// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

package main

import (
	"log"
	"os"

	"app/context"

	"syscall/zx"
	"syscall/zx/fidl"

	echo "fidl/fidl/examples/echo"
)

type echoImpl struct {
	quiet bool
}

func (echo *echoImpl) EchoString(inValue *string) (outValue *string, err error) {
	if !echo.quiet {
		log.Printf("server: %s\n", *inValue)
	}
	return inValue, nil
}

func main() {
	quiet := (len(os.Args) > 1) && os.Args[1] == "-q"
	echoService := &echo.EchoService{}
	c := context.CreateFromStartupInfo()
	c.OutgoingService.AddService(echo.EchoName, func(c zx.Channel) error {
		_, err := echoService.Add(&echoImpl{quiet}, c, nil)
		return err
	})
	fidl.Serve()
}
