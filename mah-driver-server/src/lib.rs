use interoptopus::{ffi_function, ffi_type, Inventory, InventoryBuilder, function};


#[ffi_function]
#[no_mangle]
pub extern "C" fn init() {
	todo!();
}

pub fn my_inventory() -> Inventory {
	InventoryBuilder::new()
		.register(function!(init))
		.inventory()
}