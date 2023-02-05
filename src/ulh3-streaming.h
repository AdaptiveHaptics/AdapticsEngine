#pragma once

#include <memory>
#include "ultraleap/haptics/streaming.hpp"
#include "rust/cxx.h"

using namespace Ultraleap::Haptics;

class ULHStreamingController {
public:
	ULHStreamingController(float callback_rate);
	~ULHStreamingController();
	void pause_emitter();
	void resume_emitter();
private:
	Library lib;
	StreamingEmitter emitter;
};

std::unique_ptr<ULHStreamingController> new_ulh_streaming_controller(float callback_rate);

