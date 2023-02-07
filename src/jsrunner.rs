use crate::*;
use std::{sync::mpsc, time::Duration};
use serde_v8::from_v8;
use v8::HeapStatistics;

pub enum JSCallMsg {
	EvalBatch(Vec<f64>),
	UpdatePattern(String)
}
pub enum JSReturnMsg {
	EvalBatch(Vec<EvalResults>)
}

macro_rules! pattern_eval_filename { () => { "pattern-evaluator.mjs" }; }


fn null_module_resolve_callback<'a>(
	context: v8::Local<'a, v8::Context>,
	specifier: v8::Local<'a, v8::String>,
	import_assertions: v8::Local<'a, v8::FixedArray>,
	referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
	None
}


fn to_rust_json_string<'a, T: Into<v8::Local<'a, v8::Value>>>(scope: &mut v8::HandleScope, val: T) -> String {
	v8::json::stringify(scope, val.into()).unwrap().to_rust_string_lossy(scope)
}
fn to_rust_detail_string<'a, T: Into<v8::Local<'a, v8::Value>>>(scope: &mut v8::HandleScope, val: T) -> String {
	val.into().to_detail_string(scope).unwrap().to_rust_string_lossy(scope)
}

pub fn initv8(js_call_rx: mpsc::Receiver<JSCallMsg>, js_return_tx: mpsc::SyncSender<JSReturnMsg>) {
	// v8::V8::set_flags_from_string("--expose-gc");
	// v8::V8::set_flags_from_string("--allow-natives-syntax");
	v8::V8::set_flags_from_string("--never_compact");
	let platform = v8::new_default_platform(0, false).make_shared();
	v8::V8::initialize_platform(platform.clone());
	v8::V8::initialize();

	let isolate = &mut v8::Isolate::new(Default::default());
	let scope = &mut v8::HandleScope::new(isolate);
	let context = v8::Context::new(scope);
	let scope = &mut v8::ContextScope::new(scope, context);

	let pattern_evaluator_namespace = {
		let filestr = include_str!(pattern_eval_filename!());
		let filename = v8::String::new(scope, pattern_eval_filename!()).unwrap().into();
		let v8_filestr = v8::String::new(scope, filestr).unwrap();
		let undefined = v8::undefined(scope).into();

		let script_info = v8::ScriptOrigin::new(
			scope, filename,
			0, 0, false, 0,
			undefined,
			false,
			false,
			true
		);
		let source = v8::script_compiler::Source::new(v8_filestr, Some(&script_info));
		let pattern_evaluator_script = v8::script_compiler::compile_module(scope, source).unwrap();
		pattern_evaluator_script.instantiate_module(scope, null_module_resolve_callback).unwrap();
		pattern_evaluator_script.evaluate(scope).unwrap();
		let namespace = pattern_evaluator_script.get_module_namespace().to_object(scope).unwrap();
		namespace
	};

	let pattern_eval_class = {
		let pattern_eval_class_name = v8::String::new(scope, "PatternEvaluator").unwrap();
		let pattern_eval_class = pattern_evaluator_namespace.get(scope, pattern_eval_class_name.into()).unwrap();
		pattern_eval_class
	};
	let pattern_eval_constructor: v8::Local<v8::Function> = pattern_eval_class.try_into().unwrap();

	println!("pattern_evaluator_namespace detail: {:?}", to_rust_detail_string(scope, pattern_evaluator_namespace));
	println!("pattern_eval_class detail: {:?}", pattern_eval_class.to_detail_string(scope).unwrap().to_rust_string_lossy(scope));

	let pattern_json_string_raw = r##" {"$DATA_FORMAT":"MidAirHapticsAnimationFileFormat","$REVISION":"0.0.4-alpha.1","name":"test","projection":"plane","update_rate":1,"keyframes":[{"time":0,"brush":{"brush":{"name":"circle","params":{"radius":1}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"standard","coords":{"coords":{"x":-65,"y":0,"z":0},"transition":{"name":"linear","params":{}}}},{"time":750,"brush":{"brush":{"name":"circle","params":{"radius":10}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"standard","coords":{"coords":{"x":-5,"y":55,"z":0},"transition":{"name":"linear","params":{}}}},{"time":1000,"brush":{"brush":{"name":"circle","params":{"radius":15}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"pause"},{"time":1500,"brush":{"brush":{"name":"circle","params":{"radius":8}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"standard","coords":{"coords":{"x":-5,"y":-45,"z":0},"transition":{"name":"linear","params":{}}}},{"time":1600,"brush":{"brush":{"name":"circle","params":{"radius":1}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"pause"},{"time":1700,"brush":{"brush":{"name":"line","params":{"length":1,"thickness":1,"rotation":0}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"pause"},{"time":1800,"brush":{"brush":{"name":"line","params":{"length":10,"thickness":1,"rotation":0}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"pause"},{"time":2500,"brush":{"brush":{"name":"line","params":{"length":10,"thickness":1,"rotation":360}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":1}},"transition":{"name":"linear","params":{}}},"type":"standard","coords":{"coords":{"x":55,"y":0,"z":0},"transition":{"name":"linear","params":{}}}}]} "##;
	let pattern_json_string_raw = "{\"$DATA_FORMAT\":\"MidAirHapticsAnimationFileFormat\",\"$REVISION\":\"0.0.4-alpha.1\",\"name\":\"test\",\"projection\":\"plane\",\"update_rate\":1,\"keyframes\":[{\"time\":0,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-60,\"y\":-40,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":500,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":10}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":5,\"y\":65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":1000,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2250,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-5,\"y\":-65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":2350,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2425,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":1,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2500,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":3750,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":360}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":50,\"y\":0,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}}]}";
	let pattern_json_string = v8::String::new(scope, pattern_json_string_raw).unwrap();
	let pattern_obj = v8::json::parse(scope, pattern_json_string).unwrap();
	let pattern_eval_instance = pattern_eval_constructor.new_instance(scope, &[pattern_obj]).unwrap();

	let eval_brush_at_anim_local_time = {
		let eval_brush_at_anim_local_time_name = v8::String::new(scope, "eval_brush_at_anim_local_time").unwrap();
		let rv = pattern_eval_instance.get(scope, eval_brush_at_anim_local_time_name.into()).unwrap();
		let eval_brush_at_anim_local_time: v8::Local<v8::Function> = rv.try_into().unwrap();
		eval_brush_at_anim_local_time
	};


	use std::time::Instant;
	let warmup_iterations = 5;
	let mut max_time = Duration::default();
	let platform = platform.clone();
	for o in 0..3000 {
		if (o == warmup_iterations) {
			println!("Warmup done, starting benchmark..");
			max_time = Duration::default();
		}
		let now = Instant::now();

		for i in 0..20 {
			let pattern_eval_params = v8::Object::new(scope);
			let time_str_js = v8::String::new(scope, "time").unwrap();
			let time_js = v8::Number::new(scope, f64::from(i) * 0.05);
			pattern_eval_params.set(scope, time_str_js.into(), time_js.into());
			let eval_result = eval_brush_at_anim_local_time.call(scope, pattern_eval_instance.into(), &[pattern_eval_params.into()]).unwrap();

			let result: EvalResults = from_v8(scope, eval_result).unwrap();
			if result.coords.z != 0.0 {
				println!("{:?}", result);
			}
		}
		// scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

		let elapsed = now.elapsed();
		if (elapsed > max_time) {
			max_time = elapsed;
		}
		let mut heap_stats = v8::HeapStatistics::default();
		scope.get_heap_statistics(&mut heap_stats);
		println!("Elapsed: {:.2?}, heap_stats.used_heap_size(): {:?}", elapsed, heap_stats.used_heap_size());
		// v8::Platform::run_idle_tasks(&platform, scope, 1.0);
	}
	println!("Max elapsed: {:.2?}", max_time);


}