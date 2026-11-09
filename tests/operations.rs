use bbtree::unsync::BBTreeMap;

#[test]
fn string_key_value_read_write_update_and_delete_round_trip() {
    const COUNT: u64 = 50_000;

    let mut map = BBTreeMap::<String, u64>::new();
    let keys: Vec<String> = (0..COUNT)
        .map(|index| format!("tenant/0001/key-{index:08}"))
        .collect();

    for (index, key) in keys.iter().enumerate() {
        assert_eq!(map.insert(key.clone(), index as u64), None);
    }
    assert_eq!(map.len() as u64, COUNT);

    for (index, key) in keys.iter().enumerate() {
        assert_eq!(map.get(key).copied(), Some(index as u64));
    }
    assert_eq!(map.get(&"tenant/0001/missing".to_owned()), None);

    for (index, key) in keys.iter().enumerate() {
        let value = map.get_mut(key).expect("inserted key must be readable");
        *value = (index as u64).wrapping_mul(2).wrapping_add(1);
    }
    for (index, key) in keys.iter().enumerate() {
        assert_eq!(
            map.get(key).copied(),
            Some((index as u64).wrapping_mul(2).wrapping_add(1))
        );
    }

    for index in (0..COUNT).step_by(2) {
        assert_eq!(
            map.remove(&keys[index as usize]),
            Some(index.wrapping_mul(2).wrapping_add(1))
        );
    }
    assert_eq!(map.len() as u64, COUNT / 2);

    for index in (0..COUNT).step_by(2) {
        assert_eq!(map.get(&keys[index as usize]), None);
    }
    for index in (1..COUNT).step_by(2) {
        assert_eq!(
            map.get(&keys[index as usize]).copied(),
            Some(index.wrapping_mul(2).wrapping_add(1))
        );
    }

    let remaining_keys: Vec<_> = map.keys().cloned().collect();
    assert!(remaining_keys.windows(2).all(|pair| pair[0] < pair[1]));
}
