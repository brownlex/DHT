extern crate sha1;

pub fn gen_key(ip: &String) -> [u8; 20] {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    m.update(ip.as_bytes());
    m.output(&mut buf);

    return buf;
}

//i think we need some kind of carry here?
pub fn sha_subtract(first: &[u8; 20], second: &[u8; 20]) -> [u8; 20] {
	let mut result = [0u8; 20];
	for i in 0..20 {
		result[i] = first[i].wrapping_sub(second[i]);
	}

	result
}

//returns -1 if left is smaller, 0 if same and 1 if right is smaller
pub fn compare_keys(left: [u8; 20], right: [u8; 20]) ->  i8 {
	for i in 0..20 {
		if left[i] < right[i] {return -1}
		else if left[i] > right[i] {return 1};
	}

	return 0;
}