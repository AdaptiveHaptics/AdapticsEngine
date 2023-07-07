#pragma once

extern "C" {
#include "LeapC.h"
}

#include "rust/cxx.h"

struct RawTrackingCoords;

void OpenConnectionAndStartMessagePump(rust::Fn<void(RawTrackingCoords const &)> cb_func, rust::Fn<bool()> is_done);
