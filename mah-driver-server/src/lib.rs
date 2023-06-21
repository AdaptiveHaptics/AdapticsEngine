use interoptopus::{ffi_function, ffi_type, Inventory, InventoryBuilder, function};


// mod common;
// mod threads;
// use threads::pattern::pattern_eval;
// use pattern_eval::PatternEvalUpdate;
// use threads::streaming;

#[ffi_function]
#[no_mangle]
pub extern "C" fn init() {

	// let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    // let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::unbounded();
    // let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    // let (network_send_tx, network_send_rx) = crossbeam_channel::bounded(1);

    // let (its_over_tx, its_over_rx) = crossbeam_channel::bounded(1);

	// let pattern_eval_handle = thread::Builder::new()
    //     .name("pattern-eval".to_string())
    //     .spawn(move || {
    //         println!("pattern-eval thread starting...");

    //         let res = pattern_eval::pattern_eval_loop(
    //             SECONDS_PER_NETWORK_SEND,
    //             patteval_call_rx,
    //             patteval_update_rx,
    //             patteval_return_tx,
    //             network_send_tx,
    //         );

    //         // res.unwrap();
    //         res.ok(); // ignore error, only occurs when channel disconnected

    //         println!("pattern-eval thread exiting...");
    //     })
    //     .unwrap();
}

pub fn my_inventory() -> Inventory {
	InventoryBuilder::new()
		.register(function!(init))
		.inventory()
}