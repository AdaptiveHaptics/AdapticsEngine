#pragma once

extern "C" {
#include "LeapC.h"
}

#include "rust/cxx.h"

struct LMCRawTrackingCoords;

void OpenConnectionAndStartMessagePump(rust::Fn<void(LMCRawTrackingCoords const &)> cb_func, rust::Fn<bool()> is_done);
