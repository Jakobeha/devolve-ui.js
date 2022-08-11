pub macro arrayvec {
    [$($e:expr),*] => { {
        let mut arrayvec = ::arrayvec::ArrayVec::new();
        for e in [$($e),*] {
            arrayvec.try_push(e).expect("arrayvec literal has too many elements");
        }
        arrayvec
    } }
}