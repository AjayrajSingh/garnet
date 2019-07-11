// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "zircon_platform_trace.h"

#include <lib/fit/function.h>
#include <memory>
#include <zircon/syscalls.h>

#include "magma_util/dlog.h"
#include "magma_util/macros.h"

namespace magma {

#if MAGMA_ENABLE_TRACING

static std::unique_ptr<ZirconPlatformTrace> g_platform_trace;

ZirconPlatformTrace::ZirconPlatformTrace()
    : loop_(&kAsyncLoopConfigNoAttachToThread), trace_provider_(loop_.dispatcher())
{
}

bool ZirconPlatformTrace::Initialize()
{
    zx_status_t status = loop_.StartThread();
    if (status != ZX_OK)
        return DRETF(false, "Failed to start async loop");
    return true;
}

PlatformTrace* PlatformTrace::Get()
{
    if (!g_platform_trace)
        g_platform_trace = std::make_unique<ZirconPlatformTrace>();
    return g_platform_trace.get();
}

ZirconPlatformTraceObserver::ZirconPlatformTraceObserver()
    : loop_(&kAsyncLoopConfigNoAttachToThread)
{
}

bool ZirconPlatformTraceObserver::Initialize()
{
    zx_status_t status = loop_.StartThread();
    if (status != ZX_OK)
        return DRETF(false, "Failed to start async loop");
    return true;
}

void ZirconPlatformTraceObserver::SetObserver(fit::function<void(bool)> callback)
{
    observer_.Stop();
    enabled_ = false;

    observer_.Start(loop_.dispatcher(), [this, callback = std::move(callback)] {
        bool enabled = trace_state() == TRACE_STARTED;
        if (this->enabled_ != enabled) {
            this->enabled_ = enabled;
            callback(enabled);
        }
    });
}

// static
std::unique_ptr<PlatformTraceObserver> PlatformTraceObserver::Create()
{
    auto observer = std::make_unique<ZirconPlatformTraceObserver>();
    if (!observer->Initialize())
        return nullptr;
    return observer;
}

#else

PlatformTrace* PlatformTrace::Get() { return nullptr; }

#endif

// static
uint64_t PlatformTrace::GetCurrentTicks() { return zx_ticks_get(); }

} // namespace magma
