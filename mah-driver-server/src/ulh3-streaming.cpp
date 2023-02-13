#include <chrono>

#include "ulh3-streaming.h"

#include "xk-web-midair-haptic-driver-rust/src/main.rs.h"


#define throw_if_error(res) if (!res) { throw std::exception(res.error().message()); }
void unwrap(result<void> res) {
	throw_if_error(res);
}


using JavascriptMilliseconds = std::chrono::duration<double, std::milli>;

void ecallback_shim(
	rust_ecallback cb_func,
	const StreamingEmitter& emitter,
    OutputInterval& interval,
    const LocalTimePoint& submission_deadline
) {

	std::vector<double> time_arr_ms;
    for (auto& sample : interval) {
		JavascriptMilliseconds msd = sample.time_since_epoch(); //It's important to use double to prevent precision issues with long run times
        auto ms = msd.count();
		time_arr_ms.push_back(ms);
    }
	std::vector<EvalResult> eval_results_arr(time_arr_ms.size());

	cb_func(time_arr_ms, eval_results_arr);

	int i = 0;
	for (auto& sample : interval) {
		auto eval_result = eval_results_arr.at(i);
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
	if (time_remaining < 0) {
		std::cout << "WARNING: missed deadline by " << -time_remaining << "ms" << std::endl;
	}
}


ULHStreamingController::ULHStreamingController(float callback_rate, rust_ecallback cb_func) : lib(), emitter((unwrap(lib.connect()), lib)) {
    auto device_result = lib.findDevice(DeviceFeatures::StreamingHaptics);
    throw_if_error(device_result);

	unwrap(emitter.addDevice(device_result.value()));
	unwrap(emitter.setControlPointCount(1, AdjustRate::All));
	// std::function<void(const StreamingEmitter&, OutputInterval&, const LocalTimePoint&)>
	EmissionCallbackFunction callback = std::bind(ecallback_shim, cb_func, std::placeholders::_1, std::placeholders::_2, std::placeholders::_3);
	unwrap(emitter.setEmissionCallback(std::move(callback)));
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


std::unique_ptr<ULHStreamingController> new_ulh_streaming_controller(float callback_rate, rust_ecallback cb_func) {
	return std::make_unique<ULHStreamingController>(callback_rate, cb_func);
}


double get_current_chrono_time() {
	LocalTimePoint tp = LocalTimeClock::now();
	return JavascriptMilliseconds(tp.time_since_epoch()).count();
}