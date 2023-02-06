#include <chrono>

#include "ulh3-streaming.h"
#include "xk-web-midair-haptic-driver-rust/src/main.rs.h"


#define throw_if_error(res) if (!res) { throw std::exception(res.error().message()); }
void unwrap(result<void> res) {
	throw_if_error(res);
}


using JavascriptMilliseconds = std::chrono::duration<double, std::milli>;

void ecallback_shim(const StreamingEmitter& emitter,
    OutputInterval& interval,
    const LocalTimePoint& submission_deadline
) {

	std::vector<double> time_arr_ms;
    for (auto& sample : interval) {
		JavascriptMilliseconds msd = sample.time_since_epoch(); //It's important to use double to prevent precision issues with long run times
        auto ms = msd.count();
		time_arr_ms.push_back(ms);
    }

	auto result_vec = streaming_emission_callback(time_arr_ms);

	int i = 0;
	for (auto& sample : interval) {
		auto eval_result = result_vec.at(i);
		Vector3 p;
        p.x = static_cast<float>(eval_result.coords.x);
        p.y = static_cast<float>(eval_result.coords.y);
        p.z = static_cast<float>(eval_result.coords.z);
		p.z = 0.1f; // enforce 10cm above device

        sample.controlPoint(0).setPosition(p);
        sample.controlPoint(0).setIntensity(static_cast<float>(eval_result.intensity));

		i++;
	}

	auto done_time = LocalTimeClock::now();
	auto time_remaining = std::chrono::duration_cast<JavascriptMilliseconds>(submission_deadline - done_time).count();
}


ULHStreamingController::ULHStreamingController(float callback_rate) : lib(), emitter((unwrap(lib.connect()), lib)) {
    auto device_result = lib.findDevice(DeviceFeatures::StreamingHaptics);
    throw_if_error(device_result);

	unwrap(emitter.addDevice(device_result.value()));
	unwrap(emitter.setControlPointCount(1, AdjustRate::All));
	unwrap(emitter.setEmissionCallback(&ecallback_shim));
	unwrap(emitter.setCallbackRate(callback_rate));
	emitter.start();
	emitter.pause();
}

void ULHStreamingController::pause_emitter() {
	unwrap(emitter.pause());
}
void ULHStreamingController::resume_emitter() {
	unwrap(emitter.resume());
}
size_t ULHStreamingController::getMissedCallbackIterations() const {
	auto v = emitter.getMissedCallbackIterations();
	throw_if_error(v);
	return v.value();
}

ULHStreamingController::~ULHStreamingController() {
	unwrap(emitter.stop());
	unwrap(lib.disconnect()); // unnecessary
}



std::unique_ptr<ULHStreamingController> new_ulh_streaming_controller(float callback_rate) {
	return std::make_unique<ULHStreamingController>(callback_rate);
}
