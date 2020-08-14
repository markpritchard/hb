use super::*;

#[test]
fn random_iter() {
    let iter = RandomIter::new(10, 5);
    let actual: Vec<usize> = iter.collect();
    assert_eq!(5, actual.len());
    for v in actual {
        assert!(v < 10);
    }
}

#[test]
fn sequential_iter() {
    let iter = SequentialIter::new(2, 5);
    let actual: Vec<usize> = iter.collect();
    let expected: Vec<usize> = vec!(0, 1, 0, 1, 0);
    assert_eq!(expected, actual);
}