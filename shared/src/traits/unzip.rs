// Generalized unzip
macro_rules! unzip {
    ((), ($($vs: ident: $ts: ident),*)) => {};

    (($f: ident: $tr: ident $(,$fs: ident: $trs: ident)*), ($v: ident: $t: ident $(,$vs: ident: $ts: ident)*)) => {
        unzip!(($($fs: $trs),*), ($($vs: $ts),*));

        pub trait $tr<$t $(,$ts)*> {
            fn $f(self) -> (Vec<$t> $(,Vec<$ts>)*);
        }

        impl<Type, $t $(,$ts)*> $tr<$t $(,$ts)*> for Type where Type: Iterator<Item = ($t $(,$ts)*)> {
            fn $f(self) -> (Vec<$t> $(,Vec<$ts>)*) {
                self.fold(
                    (Vec::<$t>::new() $(, Vec::<$ts>::new())*),
                    #[allow(non_snake_case)]
                    |(mut $t $(,mut $ts)*), ($v $(,$vs)*)| {
                        $t.push($v);
                        $($ts.push($vs);)*
                        ($t $(,$ts)*)
                    }
                )
            }
        }
    };
}

unzip!(
    (
        unzip8_vec: Unzip26,
        unzip8_vec: Unzip25,
        unzip8_vec: Unzip24,
        unzip8_vec: Unzip23,
        unzip8_vec: Unzip22,
        unzip8_vec: Unzip21,
        unzip8_vec: Unzip20,
        unzip8_vec: Unzip19,
        unzip8_vec: Unzip18,
        unzip8_vec: Unzip17,
        unzip8_vec: Unzip16,
        unzip8_vec: Unzip15,
        unzip8_vec: Unzip14,
        unzip8_vec: Unzip13,
        unzip8_vec: Unzip12,
        unzip8_vec: Unzip11,
        unzip8_vec: Unzip10,
        unzip8_vec: Unzip9,
        unzip8_vec: Unzip8,
        unzip7_vec: Unzip7,
        unzip6_vec: Unzip6,
        unzip5_vec: Unzip5,
        unzip4_vec: Unzip4,
        unzip3_vec: Unzip3
    ),
    (
        a: A,
        b: B,
        c: C,
        d: D,
        e: E,
        f: F,
        g: G,
        h: H,
        i: I,
        j: J,
        k: K,
        l: L,
        m: M,
        n: N,
        o: O,
        p: P,
        q: Q,
        r: R,
        s: S,
        t: T,
        u: U,
        v: V,
        w: W,
        x: X,
        y: Y,
        z: Z
    )
);
