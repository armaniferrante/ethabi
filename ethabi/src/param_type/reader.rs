use {ParamType, Error, ErrorKind};

/// Used to convert param type represented as a string to rust structure.
pub struct Reader;

impl Reader {
	/// Converts string to param type.
	pub fn read(name: &str) -> Result<ParamType, Error> {
        println!("Reading = {}", name);
		// check if it is a fixed or dynamic array.
		match name.chars().last() {
			Some(']') => {
				// take number part
				let num: String = name.chars()
					.rev()
					.skip(1)
					.take_while(|c| *c != '[')
					.collect::<String>()
					.chars()
					.rev()
					.collect();
			    let count = name.chars().count();
			    if num.is_empty() {
				    // we already know it's a dynamic array!
				    let subtype = try!(Reader::read(&name[..count - 2]));
				    return Ok(ParamType::Array(Box::new(subtype)));
			    } else {
				    // it's a fixed array.
				    let len = try!(usize::from_str_radix(&num, 10));
				    let subtype = try!(Reader::read(&name[..count - num.len() - 2]));
				    return Ok(ParamType::FixedArray(Box::new(subtype), len));
                }
			}
			Some(')') => {
                if !name.starts_with("tuple(") {
                    return Err(ErrorKind::InvalidName(name.to_owned()).into());
                }

				let mut subtypes = Vec::new();
				let mut nested = 1;//isize;
				let mut last_item = 6;

                // TODO: This is obviously shit. Clean this up.
                // Find the index of the first non-tuple element in the string.
                loop {
                    if !name[last_item..].starts_with("tuple(")  {
                        break;
                    }
                    nested += 1;
                    last_item += 6;
                }

                // Iterate through everything after tuple
                let mut pos = last_item;
                loop {
                    // Loop Header.
                    if pos >= name.len() {
                        break;
                    }
                    let mut c = name[last_item..].chars().nth(pos-last_item).unwrap();

                    // Loop body.
                    if name[pos..].starts_with("tuple(") {
                        nested += 1;
                        // Next char after 'tuple('
                        pos += 6;
                    } else {
                        match c {
						    ')' => {
							    nested -= 1;
							    if nested < 0 {
								    return Err(ErrorKind::InvalidName(name.to_owned()).into());
							    } else if nested == 0 {
								    let sub = &name[last_item..pos];
								    let subtype = Reader::read(sub)?;
								    subtypes.push(subtype);
								    last_item = pos + 1;
							    }
						    }
						    ',' if nested == 1 => {
							    let sub = &name[last_item..pos];
							    let subtype = Reader::read(sub)?;
							    subtypes.push(subtype);
							    last_item = pos + 1;
						    }
						    _ => ()
					    }
                        // Next char.
                        pos += 1;
                    }
                }

                // Use dynamic tuple if any of the inner types are dynamic.
                let mut dynamic = false;
                for t in &subtypes {
                    match t {
                        ParamType::Bytes => dynamic = true,
                        ParamType::String => dynamic = true,
                        ParamType::Array(_t) => dynamic = true,
                        ParamType::Tuple(_t) => dynamic = true,
                        _ => (),
                    };
                }
                if dynamic {
                    return Ok(ParamType::Tuple(subtypes));
                } else {
                    return Ok(ParamType::FixedTuple(subtypes));
                }
			}
			_ => ()
		}

		let result = match name {
			"address" => ParamType::Address,
			"bytes" => ParamType::Bytes,
			"bool" => ParamType::Bool,
			"string" => ParamType::String,
			"int" => ParamType::Int(256),
			"uint" => ParamType::Uint(256),
			s if s.starts_with("int") => {
				let len = try!(usize::from_str_radix(&s[3..], 10));
				ParamType::Int(len)
			},
			s if s.starts_with("uint") => {
				let len = try!(usize::from_str_radix(&s[4..], 10));
				ParamType::Uint(len)
			},
			s if s.starts_with("bytes") => {
				let len = try!(usize::from_str_radix(&s[5..], 10));
				ParamType::FixedBytes(len)
			},
			_ => {
				return Err(ErrorKind::InvalidName(name.to_owned()).into());
			}
		};

		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use ParamType;
	use super::Reader;

	#[test]
	fn test_read_param() {
		assert_eq!(Reader::read("address").unwrap(), ParamType::Address);
		assert_eq!(Reader::read("bytes").unwrap(), ParamType::Bytes);
		assert_eq!(Reader::read("bytes32").unwrap(), ParamType::FixedBytes(32));
		assert_eq!(Reader::read("bool").unwrap(), ParamType::Bool);
		assert_eq!(Reader::read("string").unwrap(), ParamType::String);
		assert_eq!(Reader::read("int").unwrap(), ParamType::Int(256));
		assert_eq!(Reader::read("uint").unwrap(), ParamType::Uint(256));
		assert_eq!(Reader::read("int32").unwrap(), ParamType::Int(32));
		assert_eq!(Reader::read("uint32").unwrap(), ParamType::Uint(32));
	}

	#[test]
	fn test_read_array_param() {
		assert_eq!(Reader::read("address[]").unwrap(), ParamType::Array(Box::new(ParamType::Address)));
		assert_eq!(Reader::read("uint[]").unwrap(), ParamType::Array(Box::new(ParamType::Uint(256))));
		assert_eq!(Reader::read("bytes[]").unwrap(), ParamType::Array(Box::new(ParamType::Bytes)));
		assert_eq!(Reader::read("bool[][]").unwrap(), ParamType::Array(Box::new(ParamType::Array(Box::new(ParamType::Bool)))));
	}

	#[test]
	fn test_read_fixed_array_param() {
		assert_eq!(Reader::read("address[2]").unwrap(), ParamType::FixedArray(Box::new(ParamType::Address), 2));
		assert_eq!(Reader::read("bool[17]").unwrap(), ParamType::FixedArray(Box::new(ParamType::Bool), 17));
		assert_eq!(Reader::read("bytes[45][3]").unwrap(), ParamType::FixedArray(Box::new(ParamType::FixedArray(Box::new(ParamType::Bytes), 45)), 3));
	}

	#[test]
	fn test_read_fixed_tuple_param() {
		assert_eq!(Reader::read("tuple(address,bool)").unwrap(), ParamType::FixedTuple(vec![ParamType::Address, ParamType::Bool]));
		assert_eq!(Reader::read("tuple(bool[3],uint256)").unwrap(), ParamType::FixedTuple(vec![ParamType::FixedArray(Box::new(ParamType::Bool), 3), ParamType::Uint(256)]));
		assert_eq!(Reader::read("tuple(address,bytes)").unwrap(), ParamType::Tuple(vec![ParamType::Address, ParamType::Bytes]));
		assert_eq!(Reader::read("tuple(bool[3],bytes)").unwrap(), ParamType::Tuple(vec![ParamType::FixedArray(Box::new(ParamType::Bool), 3), ParamType::Bytes]));
    }

	#[test]
	fn test_read_mixed_arrays() {
		assert_eq!(Reader::read("bool[][3]").unwrap(), ParamType::FixedArray(Box::new(ParamType::Array(Box::new(ParamType::Bool))), 3));
		assert_eq!(Reader::read("bool[3][]").unwrap(), ParamType::Array(Box::new(ParamType::FixedArray(Box::new(ParamType::Bool), 3))));
	}
}
