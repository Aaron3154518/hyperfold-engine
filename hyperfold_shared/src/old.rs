// Maps a list of idents (x1, x2, ..., xn) to (...((x1, x2), x3), ... xn)
#[macro_export]
macro_rules! zip_tuple {
    // Zip
    ($v1: ident) => {
        $v1
    };

    ($v1: ident, $v2: ident) => {
        ($v2, $v1)
    };

    ($v1: ident $(,$vs: ident)+) => {
        (zip_tuple!($($vs),*), $v1)
    };

    // Reverse
    ((), $($vs: ident),*) => {
        zip_tuple!($($vs),*)
    };

    (($v1: ident $(,$vs: ident)*) $(,$vs2: ident)*) => {
        zip_tuple!(($($vs),*), $v1 $(,$vs2)*)
    };
}

fn zip_tuple_example() {
    let v1 = vec!["A"];
    let v2 = vec!["B"];
    let v3 = vec!["C"];
    for zip_tuple!(x1, x2, x3) in v1.iter_mut().zip(v2.iter_mut()).zip(v3.iter_mut()) {
        println("{} {} {}", x1, x2, x3);
    }
}
