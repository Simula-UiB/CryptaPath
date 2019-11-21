use crate::soc::{utils, Id};
use std::io::Error;

#[test]
fn swap_test() {
    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let save = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let expected_result = bdd!(5;0;[("1+2",[(1;2,3)]);("0+4",[(2;5,4);(3;0,4)]);("3+2",[(4;6,0);(5;0,6)]);("",[(6;0,0)])]);
    bdd.swap(1, 2);
    assert_eq!(bdd, expected_result);
    bdd.swap(1, 2);
    assert_eq!(bdd, save);
}

#[test]
fn add_test() {
    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let save = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let expected_result = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,4);(3;4,0)]);("0+3+2+4",[(4;0,6)]);("",[(6;0,0)])]);
    bdd.add(1, 2);
    assert_eq!(bdd, expected_result);
    bdd.add(1, 2);
    assert_eq!(bdd, save);
}

#[test]
fn absorb_test() {
    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    bdd.absorb(2, false);
    let expected_result = bdd!(5;0;[("1+2",[(1;2,0)]);("3+2",[(2;0,6)]);("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);

    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    bdd.absorb(1, true);
    let expected_result = bdd!(5;0;[("1+2",[(1;5,0)]);("0+4",[(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);

    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    bdd.absorb(0, false);
    let expected_result = bdd!(5;0;[("3+2",[(2;4,5)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);
}

#[test]
fn drop_test() {
    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    bdd.drop(2);
    let expected_result = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;6,6);(3;6,0)]);("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);

    let mut bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    bdd.drop(0);
    let expected_result = bdd!(5;0;[("3+2",[(2;4,5)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);

    let mut bdd = bdd!(5;0;[("0+4",[(5;6,0)]);("",[(6;0,0)])]);
    bdd.drop(0);
    let expected_result = bdd!(5;0;[("",[(6;0,0)])]);
    assert_eq!(bdd, expected_result);
}

#[test]
fn count_path_test() {
    let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(bdd.count_paths(), 3);

    let bdd = bdd!(5;0;[("0+4",[(4;6,6)]);("",[(6;0,0)])]);
    assert_eq!(bdd.count_paths(), 2);

    let bdd = bdd!(5;0;[("",[(6;0,0)])]);
    assert_eq!(bdd.count_paths(), 0);
}

#[test]
fn join_test() -> Result<(), Error> {
    let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let bdd_2 = bdd!(5;1;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let mut system = system![bdd, bdd_2]?;
    let join_id = system.join_bdds(Id::new(0), Id::new(1))?;
    let result = system
        .pop_bdd(join_id)
        .expect("Bdd of id joined should be in the system");
    let expected_result = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,7);(5;7,0)]);
        ("1+2",[(7;8,9)]);("3+2",[(8;10,11);(9;10,0)]);("0+4",[(10;0,12);(11;12,0)]);("",[(12;0,0)])]);
    assert_eq!(result, expected_result);
    Ok(())
}
#[test]
fn join_empty_bdd() -> Result<(), Error> {
    let empty_bdd = bdd!(5;0;[("",[(1;0,0)])]);
    let bdd = bdd!(5;1;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let mut system = system![bdd, empty_bdd]?;
    let join_id = system.join_bdds(Id::new(0), Id::new(1))?;
    let result = system
        .pop_bdd(join_id)
        .expect("Bdd of id joined should be in the system");
    let expected_result = bdd!(5;1;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(result, expected_result);

    // check that bdd is unchanged when joined with an empty BDD independantly of the order
    let empty_bdd = bdd!(5;0;[("",[(1;0,0)])]);
    let bdd = bdd!(5;1;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let mut system = system![bdd, empty_bdd]?;
    let join_id = system.join_bdds(Id::new(1), Id::new(0))?;
    let result = system
        .pop_bdd(join_id)
        .expect("Bdd of id joined should be in the system");
    let expected_result = bdd!(5;1;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    assert_eq!(result, expected_result);
    Ok(())
}

#[test]
fn fix_test() -> Result<(), Error> {
    let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let mut system = system![bdd]?;
    let expected_result = bdd!(5;0;[("1+2",[(1;2,3)]);("2",[(2;5,4);(3;0,4)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    system.fix(vec![3], true)?;
    assert_eq!(
        system
            .pop_bdd(Id::new(0))
            .expect("Bdd of id 0 should be in the system"),
        expected_result
    );
    let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let expected_result = bdd!(5;0;[("1+3",[(2;4,5)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    system.push_bdd(bdd)?;
    system.fix(vec![1, 2], false)?;
    assert_eq!(
        system
            .pop_bdd(Id::new(0))
            .expect("Bdd of id 0 should be in the system"),
        expected_result
    );
    Ok(())
}

#[test]
fn test_equality() {
    let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;4,5);(3;4,0)]);("0+4",[(4;0,6);(5;6,0)]);("",[(6;0,0)])]);
    let same_bdd = bdd!(5;0;[("1+2",[(10000;20000,30000)]);("3+2",[(20000;40000,50000);(30000;40000,0)]);
    ("0+4",[(40000;0,60000);(50000;60000,0)]);("",[(60000;0,0)])]);
    assert_eq!(bdd, same_bdd)
}
