use super::*;

#[test]
fn gcd_with_zero() {
    assert_eq!(gcd(18, 0), 18);
    assert_eq!(gcd(0, 21), 21);
}

#[test]
fn gcd_order_independant() {
    assert_eq!(gcd(18, 39), gcd(39, 18));
    assert_eq!(gcd(74, 36), gcd(36, 74));
    assert_eq!(gcd(1274, 3256), gcd(3256, 1274));
}

#[test]
fn gcd_divisible() {
    assert_eq!(gcd(99, 9), 9);
    assert_eq!(gcd(9, 99), 9);

    assert_eq!(gcd(38, 19), 19);
    assert_eq!(gcd(19, 38), 19);

    assert_eq!(gcd(68, 4), 4);
    assert_eq!(gcd(4, 68), 4);
}

#[test]
fn gcd_relative_primes() {
    assert_eq!(gcd(99, 49), 1);
    assert_eq!(gcd(49, 99), 1);

    assert_eq!(gcd(5, 13), 1);
    assert_eq!(gcd(13, 5), 1);

    assert_eq!(gcd(25, 98), 1);
    assert_eq!(gcd(98, 25), 1);
}

#[test]
fn gcd_known_examples() {
    assert_eq!(gcd(25, 40), 5);
    assert_eq!(gcd(40, 25), 5);

    assert_eq!(gcd(36, 90), 18);
    assert_eq!(gcd(90, 36), 18);

    assert_eq!(gcd(26, 169), 13);
    assert_eq!(gcd(169, 26), 13);
}
