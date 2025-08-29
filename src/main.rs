use collomatique::ilp::linalg::*;

fn main() {
    let mut mat = SqMat::new(5);

    mat[(0,0)] = -5;
    mat[(2,3)] = 4242;

    println!("{}", mat);

    let mut vect = Vect::new(5);

    vect[0] = 1;
    vect[3] = 1;

    println!("{}", vect);

    println!("{}", mat*vect);
}
