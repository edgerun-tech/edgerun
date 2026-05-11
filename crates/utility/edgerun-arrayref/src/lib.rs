#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]

#[macro_export]
macro_rules! array_ref {
    ($arr:expr, $offset:expr, $len:expr) => {{
        {
            #[inline]
            const unsafe fn as_array<T>(slice: &[T]) -> &[T; $len] {
                // SAFETY: caller guarantees bounds
                unsafe { &*(slice.as_ptr() as *const [_; $len]) }
            }
            let offset = $offset;
            let slice = &$arr[offset..offset + $len];
            #[allow(unused_unsafe)]
            unsafe {
                as_array(slice)
            }
        }
    }};
}

#[macro_export]
macro_rules! array_refs {
    ( $arr:expr, $( $pre:expr ),* ; .. ;  $( $post:expr ),* ) => {{
        {
            use core::slice;
            #[inline]
            #[allow(unused_assignments)]
            const unsafe fn as_arrays<T>(a: &[T]) -> ( $( &[T; $pre], )* &[T],  $( &[T; $post], )*) {
                const MIN_LEN: usize = 0usize $( .saturating_add($pre) )* $( .saturating_add($post) )*;
                let var_len = a.len() - MIN_LEN;
                let mut p = a.as_ptr();
                ( $( {
                    let aref = & *(p as *const [T; $pre]);
                    p = p.add($pre);
                    aref
                }, )* {
                    let sl = slice::from_raw_parts(p as *const T, var_len);
                    p = p.add(var_len);
                    sl
                }, $( {
                    let aref = & *(p as *const [T; $post]);
                    p = p.add($post);
                    aref
                }, )*)
            }
            let input = $arr;
            #[allow(unused_unsafe)]
            unsafe {
                as_arrays(input)
            }
        }
    }};
    ( $arr:expr, $( $len:expr ),* ) => {{
        {
            #[inline]
            #[allow(unused_assignments)]
            const unsafe fn as_arrays<T>(a: &[T; $( $len + )* 0 ]) -> ( $( &[T; $len], )* ) {
                let mut p = a.as_ptr();
                ( $( {
                    let aref = &*(p as *const [T; $len]);
                    p = p.offset($len as isize);
                    aref
                }, )* )
            }
            let input = $arr;
            #[allow(unused_unsafe)]
            unsafe {
                as_arrays(input)
            }
        }
    }}
}

#[macro_export]
macro_rules! mut_array_refs {
    ( $arr:expr, $( $pre:expr ),* ; .. ;  $( $post:expr ),* ) => {{
        {
            use core::slice;
            #[inline]
            #[allow(unused_assignments)]
            unsafe fn as_arrays<T>(a: &mut [T]) -> ( $( &mut [T; $pre], )* &mut [T],  $( &mut [T; $post], )*) {
                const MIN_LEN: usize = 0usize $( .saturating_add($pre) )* $( .saturating_add($post) )*;
                let var_len = a.len() - MIN_LEN;
                let mut p = a.as_mut_ptr();
                ( $( {
                    let aref = &mut *(p as *mut [T; $pre]);
                    p = p.add($pre);
                    aref
                }, )* {
                    let sl = slice::from_raw_parts_mut(p as *mut T, var_len);
                    p = p.add(var_len);
                    sl
                }, $( {
                    let aref = &mut *(p as *mut [T; $post]);
                    p = p.add($post);
                    aref
                }, )*)
            }
            let input = $arr;
            #[allow(unused_unsafe)]
            unsafe {
                as_arrays(input)
            }
        }
    }};
    ( $arr:expr, $( $len:expr ),* ) => {{
        {
            #[inline]
            #[allow(unused_assignments)]
            unsafe fn as_arrays<T>(a: &mut [T; $( $len + )* 0 ]) -> ( $( &mut [T; $len], )* ) {
                let mut p = a.as_mut_ptr();
                ( $( {
                    let aref = &mut *(p as *mut [T; $len]);
                    p = p.add($len);
                    aref
                }, )* )
            }
            let input = $arr;
            #[allow(unused_unsafe)]
            unsafe {
                as_arrays(input)
            }
        }
    }};
}

#[macro_export]
macro_rules! array_mut_ref {
    ($arr:expr, $offset:expr, $len:expr) => {{
        {
            #[inline]
            unsafe fn as_array<T>(slice: &mut [T]) -> &mut [T; $len] {
                &mut *(slice.as_mut_ptr() as *mut [_; $len])
            }
            let offset = $offset;
            let slice = &mut $arr[offset..offset + $len];
            #[allow(unused_unsafe)]
            unsafe {
                as_array(slice)
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple_case_works() {
        fn check(expected: [u8; 3], actual: &[u8; 3]) {
            for (e, a) in (&expected).iter().zip(actual.iter()) {
                assert_eq!(e, a)
            }
        }
        let mut foo: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        {
            let bar = array_ref!(foo, 2, 3);
            check([2, 3, 4], bar);
        }
        check([0, 1, 2], array_ref!(foo, 0, 3));
        fn zero2(x: &mut [u8; 2]) {
            x[0] = 0;
            x[1] = 0;
        }
        zero2(array_mut_ref!(foo, 8, 2));
        check([0, 0, 10], array_ref!(foo, 8, 3));
    }

    #[test]
    fn test_array_refs() {
        let mut data: [usize; 128] = [0; 128];
        for i in 0..128 {
            data[i] = i;
        }
        let data = data;
        let (a, b, c, d, e) = array_refs!(&data, 1, 14, 3, 100, 10);
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 14);
        assert_eq!(c.len(), 3);
        assert_eq!(d.len(), 100);
        assert_eq!(e.len(), 10);
        assert_eq!(a, array_ref![data, 0, 1]);
        assert_eq!(b, array_ref![data, 1, 14]);
        assert_eq!(c, array_ref![data, 15, 3]);
        assert_eq!(e, array_ref![data, 118, 10]);
    }

    #[test]
    fn test_mut_array_refs() {
        let mut data: [usize; 128] = [0; 128];
        {
            let (a, b, c, d, e) = mut_array_refs!(&mut data, 1, 14, 3, 100, 10);
            assert_eq!(a.len(), 1);
            assert_eq!(b.len(), 14);
            assert_eq!(c.len(), 3);
            assert_eq!(d.len(), 100);
            assert_eq!(e.len(), 10);
            *a = [1; 1];
            *b = [14; 14];
            *c = [3; 3];
            *d = [100; 100];
            *e = [10; 10];
        }
        assert_eq!(&[1; 1], array_ref![data, 0, 1]);
        assert_eq!(&[14; 14], array_ref![data, 1, 14]);
        assert_eq!(&[3; 3], array_ref![data, 15, 3]);
        assert_eq!(&[10; 10], array_ref![data, 118, 10]);
    }

    #[test]
    fn test_array_refs_dotdot() {
        let mut data: [usize; 128] = [0; 128];
        for i in 0..128 {
            data[i] = i;
        }
        let data = data;
        let (a, b, c, d, e) = array_refs!(&data, 1, 14, 3; ..; 10);
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 14);
        assert_eq!(c.len(), 3);
        assert_eq!(d.len(), 100);
        assert_eq!(e.len(), 10);
        assert_eq!(a, array_ref![data, 0, 1]);
        assert_eq!(b, array_ref![data, 1, 14]);
        assert_eq!(c, array_ref![data, 15, 3]);
        assert_eq!(e, array_ref![data, 118, 10]);
    }

    #[test]
    fn test_mut_array_refs_dotdot() {
        let mut data: [usize; 128] = [0; 128];
        {
            let (a, b, c, d, e) = mut_array_refs!(&mut data, 1, 14, 3; ..; 10);
            assert_eq!(a.len(), 1);
            assert_eq!(b.len(), 14);
            assert_eq!(c.len(), 3);
            assert_eq!(d.len(), 100);
            assert_eq!(e.len(), 10);
            *a = [1; 1];
            *b = [14; 14];
            *c = [3; 3];
            *e = [10; 10];
        }
        assert_eq!(&[1; 1], array_ref![data, 0, 1]);
        assert_eq!(&[14; 14], array_ref![data, 1, 14]);
        assert_eq!(&[3; 3], array_ref![data, 15, 3]);
        assert_eq!(&[10; 10], array_ref![data, 118, 10]);
    }

    #[test]
    #[should_panic]
    fn checks_bounds() {
        let foo: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let _bar = array_ref!(foo, 1, 11);
    }
}
