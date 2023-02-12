#pragma once


#include <chrono>
#include <memory>
#include "ultraleap/haptics/streaming.hpp"
#include "rust/cxx.h"

struct EvalResult;

using namespace Ultraleap::Haptics;

using rust_ecallback = rust::Fn<void(std::vector<double> const &, std::vector<EvalResult> &)>;

class ULHStreamingController {
public:
	ULHStreamingController(float callback_rate, rust_ecallback cb_func);
	~ULHStreamingController();
	void pause_emitter();
	void resume_emitter();
	size_t getMissedCallbackIterations() const;
private:
	Library lib;
	StreamingEmitter emitter;
	rust_ecallback cb_func;
};

std::unique_ptr<ULHStreamingController> new_ulh_streaming_controller(float callback_rate, rust_ecallback cb_func);


