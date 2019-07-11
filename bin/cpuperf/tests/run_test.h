// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef GARNET_BIN_CPUPERF_TESTS_RUN_TEST_H_
#define GARNET_BIN_CPUPERF_TESTS_RUN_TEST_H_

#include <string>
#include <vector>

#include <lib/zx/job.h>
#include <lib/zx/process.h>
#include <lib/zx/time.h>

// For now don't run longer than this. The CQ bot has this timeout as well,
// so this is as good a value as any. Later we might want to add a timeout
// value to tspecs.
constexpr zx_duration_t kTestTimeout = ZX_SEC(60);

bool RunSpec(const std::string& spec_file_path);

#endif  // GARNET_BIN_CPUPERF_TESTS_RUN_TEST_H_
