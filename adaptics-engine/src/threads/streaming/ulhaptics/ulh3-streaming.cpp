#include <chrono>

#include "ulh3-streaming.h"

#include "adaptics-engine/src/threads/streaming/ulhaptics/ffi.rs.h"


#define throw_if_error(res) if (!res) { throw std::exception(res.error().message()); }
void unwrap(result<void> res) {
	throw_if_error(res);
}



/*
// for debugging
struct CircleData
{
    double radius;
    double control_point_speed;
    float control_point_intensity;
    LocalTimePoint start_time;
};
CircleData circle_data_g{ 0.02, 8.0, 1.0, LocalTimeClock::now() };
CircleData* circle_data_ptr = &circle_data_g;
void circle_callback_fordebug(const StreamingEmitter& emitter,
    OutputInterval& interval,
    const LocalTimePoint& submission_deadline)
{
    printf(".");
    auto circle_data = circle_data_ptr;
    double angular_frequency = circle_data->control_point_speed / circle_data->radius;

    for (auto& sample : interval) {
        std::chrono::duration<double> t = sample - circle_data->start_time;
        double angle = t.count() * angular_frequency;

        Vector3 p;
        p.x = static_cast<float>(std::cos(angle) * circle_data->radius);
        p.y = static_cast<float>(std::sin(angle) * circle_data->radius);
        p.z = 0.2f;

        sample.controlPoint(0).setPosition(p);
        sample.controlPoint(0).setIntensity(circle_data->control_point_intensity);
    }
}
*/

using JavascriptMilliseconds = std::chrono::duration<double, std::milli>;

void ecallback_shim(
	rust_ecallback cb_func,
	const StreamingEmitter& emitter,
    OutputInterval& interval,
    const LocalTimePoint& submission_deadline
) {
	// printf(":");
    // return circle_callback_fordebug(emitter, interval, submission_deadline);
	std::vector<double> time_arr_ms;
	std::vector<Ultraleap::Haptics::TimePointOnOutputInterval> sample_arr; // apparently we cant iterate through interval twice (i swear we used to be able to)
    for (auto& sample : interval) {
		sample_arr.push_back(sample);
		JavascriptMilliseconds msd = sample.time_since_epoch(); //It's important to use double to prevent precision issues with long run times
        auto ms = msd.count();
		time_arr_ms.push_back(ms);
    }

	std::vector<EvalResult> eval_results_arr(time_arr_ms.size());

	cb_func(time_arr_ms, eval_results_arr);

	int i = 0;
	for (auto& sample : sample_arr) {
		auto eval_result = eval_results_arr.at(i);
		Vector3 p;
        p.x = static_cast<float>(eval_result.coords.x);
        p.y = static_cast<float>(eval_result.coords.y);
        p.z = static_cast<float>(eval_result.coords.z);
		// p.z = 0.1f; // enforce 10cm above device

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
	auto device = device_result.value();

 	// auto transform_result = device.getKitTransform();
	// throw_if_error(transform_result);
	// Transform kit_transform = transform_result.value();

	unwrap(emitter.addDevice(device));
	unwrap(emitter.setControlPointCount(1, AdjustRate::All));
	// std::function<void(const StreamingEmitter&, OutputInterval&, const LocalTimePoint&)>
	EmissionCallbackFunction callback = std::bind(ecallback_shim, cb_func, std::placeholders::_1, std::placeholders::_2, std::placeholders::_3);
	unwrap(emitter.setEmissionCallback(std::move(callback)));
	unwrap(emitter.setCallbackRate(callback_rate));
	emitter.start();
	emitter.pause();
}

void ULHStreamingController::pause_emitter() {
    // printf("pause_emitter\n");
	unwrap(emitter.pause());
}
void ULHStreamingController::resume_emitter() {
    // printf("resume_emitter\n");
	unwrap(emitter.resume());
}
size_t ULHStreamingController::getMissedCallbackIterations() const {
	auto v = emitter.getMissedCallbackIterations();
	throw_if_error(v);
	return v.value();
}

ULHStreamingController::~ULHStreamingController() {
    printf("ULHStreamingController destructor\n");
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