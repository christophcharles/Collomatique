#[cfg(test)]
mod tests;

pub fn gcd(mut n1: u32, mut n2: u32) -> u32 {
    while n2 != 0 {
        let r = n1 % n2;
        n1 = n2;
        n2 = r;
    }

    n1
}

pub fn lcm(n1: u32, n2: u32) -> u32 {
    let d = gcd(n1, n2);
    n1 * n2 / d
}
