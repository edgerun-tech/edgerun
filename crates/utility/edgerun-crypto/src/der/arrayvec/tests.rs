use super::ArrayVec;
use crate::der::ErrorKind;

#[test]
fn add() {
    let mut vec = ArrayVec::<u8, 3>::new();
    vec.push(1).unwrap();
    vec.push(2).unwrap();
    vec.push(3).unwrap();

    assert_eq!(vec.push(4).err().unwrap(), ErrorKind::Overlength.into());
    assert_eq!(vec.len(), 3);
}
