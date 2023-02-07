use std::sync::mpsc;

pub enum JSCallMsg {
	EvalBatch(Vec<f64>)
}
pub enum JSReturnMsg {
	EvalBatch(Vec<f64>)
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

pub fn initv8(js_call_rx: mpsc::Receiver<JSCallMsg>, js_return_tx: mpsc::SyncSender<JSReturnMsg>) {
	let platform = v8::new_default_platform(0, false).make_shared();
	v8::V8::initialize_platform(platform);
	v8::V8::initialize();

	let isolate = &mut v8::Isolate::new(Default::default());

	let scope = &mut v8::HandleScope::new(isolate);
	let context = v8::Context::new(scope);
	let scope = &mut v8::ContextScope::new(scope, context);

	let pattern_eval_str = include_str!(pattern_eval_filename!());
	let pattern_eval_filename = v8::String::new(scope, pattern_eval_filename!()).unwrap().into();
	let pattern_eval_v8_str = v8::String::new(scope, pattern_eval_str).unwrap();
	// println!("pattern_eval_v8_str: {}", pattern_eval_v8_str.to_rust_string_lossy(scope));
	let undefined = v8::undefined(scope).into();

	let script_info = v8::ScriptOrigin::new(
		scope, pattern_eval_filename,
		0, 0, false, 0,
		undefined,
		false,
		false,
		true
	);
	let source = v8::script_compiler::Source::new(pattern_eval_v8_str, Some(&script_info));
	let script = v8::script_compiler::compile_module(scope, source).unwrap();
	script.instantiate_module(scope, null_module_resolve_callback).unwrap();
	let result = script.evaluate(scope).unwrap();
	println!("result: {:?}, {:?}", result, result.to_rust_string_lossy(scope));
	let namespace = script.get_module_namespace();
	println!("namespace: {:?}, {:?}",
		namespace.to_object(scope).unwrap().get_own_property_names(scope, v8::GetPropertyNamesArgs::default()).unwrap().to_rust_string_lossy(scope),
		namespace.to_rust_string_lossy(scope));
}