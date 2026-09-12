#![no_main]

use libfuzzer_sys::fuzz_target;
use payments::structs::money::Money;

fuzz_target!(|data: &[u8]| {
    let text = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return,
    };
    let parsed = match Money::parse(text) {
        Ok(money) => money,
        Err(_) => return,
    };
    let shown = format!("{parsed}");
    let reparsed = match Money::parse(&shown) {
        Ok(money) => money,
        Err(e) => panic!("money {shown:?} from {text:?} does not reparse: {e}"),
    };
    assert_eq!(parsed.0, reparsed.0, "round trip changed {text:?} via {shown:?}");
    let again = format!("{reparsed}");
    assert_eq!(shown, again, "display is not stable for {text:?}");
});
