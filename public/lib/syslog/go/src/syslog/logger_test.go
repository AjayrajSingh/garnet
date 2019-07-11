// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

package syslog_test

import (
	"encoding/binary"
	"fmt"
	"io/ioutil"
	"os"
	"strings"
	"syscall/zx"
	"syslog"
	"testing"
	"unicode/utf8"
)

var pid = uint64(os.Getpid())

func TestLogSimple(t *testing.T) {
	sin, sout, err := zx.NewSocket(zx.SocketDatagram)
	if err != nil {
		t.Fatal(err)
	}
	tmpFile, err := ioutil.TempFile("", "test")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpFile.Name())
	for _, logToSocket := range []bool{false, true} {
		options :=
			syslog.LogInitOptions{
				MinSeverityForFileAndLineInfo: syslog.ErrorLevel,
				Writer: tmpFile,
			}
		if logToSocket {
			options.Socket = sout
		}
		logger, err := syslog.NewLogger(options)
		if err != nil {
			t.Fatal(err)
		}
		const format = "integer: %d"
		logger.Infof(format, 10)
		if logToSocket {
			expectedMsg := fmt.Sprintf(format, 10)
			checkoutput(t, sin, expectedMsg, syslog.InfoLevel)
		} else {
			tmpFile.Sync()
			tmpFile.Seek(0, 0)
			expected := "INFO: integer: 10\n"
			b, err := ioutil.ReadAll(tmpFile)
			if err != nil {
				t.Fatal(err)
			}
			got := string(b)
			if !strings.HasSuffix(got, expected) {
				t.Errorf("%q should have ended in %q", got, expected)
			}
			if !strings.Contains(got, fmt.Sprintf("[%d]", pid)) {
				t.Errorf("%q should contains %d", got, pid)
			}
			tmpFile.Truncate(0)
		}
	}
}
func setup(t *testing.T, tags ...string) (zx.Socket, *syslog.Logger) {
	sin, sout, err := zx.NewSocket(zx.SocketDatagram)
	if err != nil {
		t.Fatal(err)
	}
	log, err := syslog.NewLogger(syslog.LogInitOptions{
		MinSeverityForFileAndLineInfo: syslog.ErrorLevel,
		Socket: sout,
		Tags:   tags,
	})
	if err != nil {
		t.Fatal(err)
	}
	return sin, log
}

func checkoutput(t *testing.T, sin zx.Socket, expectedMsg string, severity syslog.LogLevel, tags ...string) {
	var data [syslog.SocketBufferLength]byte
	if n, err := sin.Read(data[:], 0); err != nil {
		t.Fatal(err)
	} else {
		if n <= 32 {
			t.Fatalf("got invalid data: %v", data[:n])
		}
		gotpid := binary.LittleEndian.Uint64(data[0:8])
		gotTid := binary.LittleEndian.Uint64(data[8:16])
		gotTime := binary.LittleEndian.Uint64(data[16:24])
		gotSeverity := int32(binary.LittleEndian.Uint32(data[24:28]))
		gotDroppedLogs := int32(binary.LittleEndian.Uint32(data[28:32]))

		if pid != gotpid {
			t.Errorf("pid error, got: %d, want: %d", gotpid, pid)
		}

		if 0 != gotTid {
			t.Errorf("tid error, got: %d, want: %d", gotTid, 0)
		}

		if int32(severity) != gotSeverity {
			t.Errorf("severity error, got: %d, want: %d", gotSeverity, severity)
		}

		if gotTime <= 0 {
			t.Errorf("time %d should be greater than zero", gotTime)
		}

		if 0 != gotDroppedLogs {
			t.Errorf("dropped logs error, got: %d, want: %d", gotDroppedLogs, 0)
		}
		pos := 32
		for i, tag := range tags {
			length := len(tag)
			if data[pos] != byte(length) {
				t.Fatalf("tag iteration %d: expected data to be %d at pos %d, got %d", i, length, pos, data[pos])
			}
			pos = pos + 1
			gotTag := string(data[pos : pos+length])
			if tag != gotTag {
				t.Fatalf("tag iteration %d: expected tag %q , got %q", i, tag, gotTag)
			}
			pos = pos + length
		}

		if data[pos] != 0 {
			t.Fatalf("byte before msg start should be zero, got: %d, %v", data[pos], data[32:n])
		}

		msgGot := string(data[pos+1 : n-1])
		if expectedMsg != msgGot {
			t.Fatalf("expected msg:%q, got %q", expectedMsg, msgGot)
		}
		if data[n-1] != 0 {
			t.Fatalf("last byte should be zero, got: %d, %v", data[pos], data[32:n])
		}
	}
}

func TestLogWithLocalTag(t *testing.T) {
	sin, log := setup(t)
	format := "integer: %d"
	log.InfoTf("local_tag", format, 10)
	expectedMsg := fmt.Sprintf(format, 10)
	checkoutput(t, sin, expectedMsg, syslog.InfoLevel, "local_tag")
}

func TestLogWithGlobalTags(t *testing.T) {
	sin, log := setup(t, "gtag1", "gtag2")
	format := "integer: %d"
	log.InfoTf("local_tag", format, 10)
	expectedMsg := fmt.Sprintf(format, 10)
	checkoutput(t, sin, expectedMsg, syslog.InfoLevel, "gtag1", "gtag2", "local_tag")
}

func TestLoggerSeverity(t *testing.T) {
	sin, log := setup(t)
	format := "integer: %d"
	log.SetSeverity(syslog.WarningLevel)
	log.Infof(format, 10)
	_, err := sin.Read(make([]byte, 0), 0)
	if err, ok := err.(*zx.Error); !ok || err.Status != zx.ErrShouldWait {
		t.Fatal(err)
	}
	log.Warnf(format, 10)
	expectedMsg := fmt.Sprintf(format, 10)
	checkoutput(t, sin, expectedMsg, syslog.WarningLevel)
}

func TestLoggerVerbosity(t *testing.T) {
	sin, log := setup(t)
	format := "integer: %d"
	log.VLogf(syslog.TraceVerbosity, format, 10)
	_, err := sin.Read(make([]byte, 0), 0)
	if err, ok := err.(*zx.Error); !ok || err.Status != zx.ErrShouldWait {
		t.Fatal(err)
	}
	log.SetVerbosity(2)
	log.VLogf(syslog.TraceVerbosity, format, 10)
	expectedMsg := fmt.Sprintf(format, 10)
	checkoutput(t, sin, expectedMsg, syslog.LogLevel(-2))
}

func TestGlobalTagLimits(t *testing.T) {
	options := syslog.LogInitOptions{
		Writer: os.Stdout,
	}
	var tags [syslog.MaxGlobalTags + 1]string
	for i := 0; i < len(tags); i++ {
		tags[i] = "a"
	}
	options.Tags = tags[:]
	if _, err := syslog.NewLogger(options); err == nil || !strings.Contains(err.Error(), "too many tags") {
		t.Fatalf("unexpected error: %s", err)
	}
	options.Tags = tags[:syslog.MaxGlobalTags]
	var tag [syslog.MaxTagLength + 1]byte
	for i := 0; i < len(tag); i++ {
		tag[i] = 65
	}
	options.Tags[1] = string(tag[:])
	if _, err := syslog.NewLogger(options); err == nil || !strings.Contains(err.Error(), "tag too long") {
		t.Fatalf("unexpected error: %s", err)
	}
}

func TestLocalTagLimits(t *testing.T) {
	sin, log := setup(t)
	format := "integer: %d"
	var tag [syslog.MaxTagLength + 1]byte
	for i := 0; i < len(tag); i++ {
		tag[i] = 65
	}
	log.InfoTf(string(tag[:]), format, 10)
	expectedMsg := fmt.Sprintf(format, 10)
	checkoutput(t, sin, expectedMsg, syslog.InfoLevel, string(tag[:syslog.MaxTagLength]))
}

func TestLogToWriterWhenSocketCloses(t *testing.T) {
	sin, log := setup(t, "gtag1", "gtag2")
	sin.Close()
	old := os.Stderr
	f, err := ioutil.TempFile("", "stderr")
	if err != nil {
		t.Fatal(err)
	}
	os.Stderr = f
	defer func() {
		os.Stderr = old
		os.Remove(f.Name())
	}()
	format := "integer: %d"
	log.InfoTf("local_tag", format, 10)
	expectedMsg := fmt.Sprintf("[0][gtag1, gtag2, local_tag] INFO: %s\n", fmt.Sprintf(format, 10))
	content, err := ioutil.ReadFile(f.Name())
	if err != nil {
		t.Fatal(err)
	}
	got := string(content)
	if !strings.HasSuffix(got, expectedMsg) {
		t.Fatalf("%q should have ended in %q", got, expectedMsg)
	}
}

func TestMessageLenLimit(t *testing.T) {
	sin, log := setup(t)
	// 1 for starting and ending null bytes.
	msgLen := syslog.SocketBufferLength - 32 - 1 - 1

	const stripped = '𠜎'
	// Ensure only part of stripped fits.
	msg := strings.Repeat("x", msgLen-(utf8.RuneLen(stripped)-1)) + string(stripped)
	switch err := log.Infof(msg).(type) {
	case *syslog.ErrMsgTooLong:
		if err.Msg != string(stripped) {
			t.Fatalf("unexpected truncation: %s", err.Msg)
		}
	default:
		t.Fatalf("unexpected error: %v", err)
	}

	const ellipsis = "..."
	expectedMsg := msg[:msgLen-len(ellipsis)] + ellipsis
	checkoutput(t, sin, expectedMsg, syslog.InfoLevel)
}
