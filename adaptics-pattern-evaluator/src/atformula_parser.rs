use crate::ATFormula;

#[derive(Debug, Clone, PartialEq)]
pub enum ATFormulaParsingError {
	UnbalancedParenthesis,
	UnbalancedParameterQuotes,
	UnexpectedToken(Option<ATFormulaToken>),
}
impl std::fmt::Display for ATFormulaParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for ATFormulaParsingError {}


#[derive(Debug, Clone, PartialEq)]
pub enum ATFormulaToken {
	Number(f64),
	Parameter(String),
	Add,
	Subtract,
	Multiply,
	Divide,
	LeftParenthesis,
	RightParenthesis,
}

fn tokenize(formula_str: &str) -> Result<Vec<ATFormulaToken>, ATFormulaParsingError> {
	fn char_to_token(c: char) -> Option<ATFormulaToken> {
		match c {
			'+' => Some(ATFormulaToken::Add),
			'-' => Some(ATFormulaToken::Subtract),
			'*' => Some(ATFormulaToken::Multiply),
			'/' => Some(ATFormulaToken::Divide),
			'(' => Some(ATFormulaToken::LeftParenthesis),
			')' => Some(ATFormulaToken::RightParenthesis),
			_ => None,
		}
	}
	fn string_to_token(current_token: &str) -> ATFormulaToken {
		//try number, if not a number, then it's a parameter
		if let Ok(number) = current_token.parse::<f64>() {
			ATFormulaToken::Number(number)
		} else {
			ATFormulaToken::Parameter(current_token.to_string())
		}
	}
	let mut tokens = Vec::new();
	let mut current_token = String::new();

	let mut in_parameter_quotes = false;
	for c in formula_str.chars() {
		if in_parameter_quotes {
			if c == '`' {
				in_parameter_quotes = false;
				tokens.push(ATFormulaToken::Parameter(current_token.clone()));
				current_token.clear();
			} else {
				current_token.push(c);
			}
		} else { // normal parsing
			if c == '`' { in_parameter_quotes = true }
			else if let Some(ct) = char_to_token(c) {
				if !current_token.is_empty() { //if there is anything in current token, try to parse and push it
					tokens.push(string_to_token(&current_token));
					current_token.clear();
				}

				tokens.push(ct);
			} else if c == ' ' {
				//ignore
			} else {
				current_token.push(c);
			}
		}
	}
	if !current_token.is_empty() {
		if in_parameter_quotes {
			return Err(ATFormulaParsingError::UnbalancedParameterQuotes);
		}
		tokens.push(string_to_token(&current_token));
	}

	Ok(tokens)
}


type TokenStream<'a> = std::iter::Peekable<std::slice::Iter<'a, ATFormulaToken>>;

fn parse(tokens: &[ATFormulaToken]) -> Result<ATFormula, ATFormulaParsingError> {
    let mut iter = tokens.iter().peekable();
    let formula = parse_expr(&mut iter)?;
	if iter.peek().is_some() {
		return Err(ATFormulaParsingError::UnexpectedToken(iter.next().cloned()));
	}

	Ok(formula)
}

fn parse_expr(iter: &mut TokenStream) -> Result<ATFormula, ATFormulaParsingError> {
    let lhs = parse_term(iter)?;
    parse_expr_rhs(lhs, iter)
}

fn parse_expr_rhs(lhs: ATFormula, iter: &mut TokenStream) -> Result<ATFormula, ATFormulaParsingError> {
    match iter.peek() {
        Some(&&ATFormulaToken::Add) => {
            iter.next();
            let rhs = parse_term(iter)?;
            parse_expr_rhs(ATFormula::Add(Box::new(lhs), Box::new(rhs)), iter)
        }
        Some(&&ATFormulaToken::Subtract) => {
            iter.next();
            let rhs = parse_term(iter)?;
            parse_expr_rhs(ATFormula::Subtract(Box::new(lhs), Box::new(rhs)), iter)
        }
        _ => Ok(lhs),
    }
}

fn parse_term(iter: &mut TokenStream) -> Result<ATFormula, ATFormulaParsingError> {
    let lhs = parse_factor(iter)?;
    parse_term_rhs(lhs, iter)
}

fn parse_term_rhs(lhs: ATFormula, iter: &mut TokenStream) -> Result<ATFormula, ATFormulaParsingError> {
    match iter.peek() {
        Some(&&ATFormulaToken::Multiply) => {
            iter.next();
            let rhs = parse_factor(iter)?;
            parse_term_rhs(ATFormula::Multiply(Box::new(lhs), Box::new(rhs)), iter)
        }
        Some(&&ATFormulaToken::Divide) => {
            iter.next();
            let rhs = parse_factor(iter)?;
            parse_term_rhs(ATFormula::Divide(Box::new(lhs), Box::new(rhs)), iter)
        }
        _ => Ok(lhs),
    }
}

fn parse_factor(iter: &mut TokenStream) -> Result<ATFormula, ATFormulaParsingError> {
    match iter.next() {
        Some(&ATFormulaToken::Number(n)) => Ok(ATFormula::Constant(n)),
        Some(ATFormulaToken::Parameter(p)) => Ok(ATFormula::Parameter(p.clone())),
        Some(&ATFormulaToken::LeftParenthesis) => {
            let expr = parse_expr(iter)?;
            match iter.next() {
                Some(&ATFormulaToken::RightParenthesis) => Ok(expr),
                _ => Err(ATFormulaParsingError::UnbalancedParenthesis),
            }
        }
        token => Err(ATFormulaParsingError::UnexpectedToken(token.cloned())),
    }
}

pub fn parse_formula(formula_str: &str) -> Result<ATFormula, ATFormulaParsingError> {
	let tokens = tokenize(formula_str)?;
	parse(&tokens)
}


impl ATFormula {
	pub fn to_formula_string(&self) -> String {
        match self {
            ATFormula::Constant(f) => f.to_string(),
            ATFormula::Parameter(s) => format!("`{}`", s),
            ATFormula::Add(left, right) => format!("{} + {}", left.to_formula_string(), right.to_formula_string()),
            ATFormula::Subtract(left, right) => format!("{} - {}", left.to_formula_string(), right.to_formula_string()),
            ATFormula::Multiply(left, right) => format!("{} * {}", left.wrap_if_needed(), right.wrap_if_needed()),
            ATFormula::Divide(left, right) => format!("{} / {}", left.wrap_if_needed(), right.wrap_if_needed()),
        }
    }
	fn wrap_if_needed(&self) -> String {
		match self {
			ATFormula::Add(..) | ATFormula::Subtract(..) => format!("({})", self.to_formula_string()),
			_ => self.to_formula_string(),
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tokenize_simple() {
		let tokens = tokenize("1.0 + 2").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Number(2.0),
		]);
	}


	#[test]
	fn test_tokenize_with_parameters() {
		let tokens = tokenize("1 + `parameter`").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Parameter("parameter".to_string()),
		]);
	}

	#[test]
	fn test_tokenize_with_unquoted_parameters() {
		let tokens = tokenize("1 + parameter").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Parameter("parameter".to_string()),
		]);
	}

	#[test]
	fn test_tokenize_with_unquoted_parameters_leading_trailing_digits() {
		let tokens = tokenize("1+1para_meter2+4").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Parameter("1para_meter2".to_string()),
			ATFormulaToken::Add,
			ATFormulaToken::Number(4.0),
		]);
	}

	#[test]
	fn test_tokenize_parameters_containing_special() {
		let tokens = tokenize("1 + `a + 2`").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Parameter("a + 2".to_string()),
		]);
	}

	#[test]
	fn test_tokenize_with_parentheses() {
		let tokens = tokenize("1 + (2 - 3)").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::LeftParenthesis,
			ATFormulaToken::Number(2.0),
			ATFormulaToken::Subtract,
			ATFormulaToken::Number(3.0),
			ATFormulaToken::RightParenthesis,
		]);
	}

	#[test]
	fn test_tokenize_with_parameters_and_parentheses() {
		let tokens = tokenize("1 + `parameter` + (2 - 3)").unwrap();
		assert_eq!(tokens, vec![
			ATFormulaToken::Number(1.0),
			ATFormulaToken::Add,
			ATFormulaToken::Parameter("parameter".to_string()),
			ATFormulaToken::Add,
			ATFormulaToken::LeftParenthesis,
			ATFormulaToken::Number(2.0),
			ATFormulaToken::Subtract,
			ATFormulaToken::Number(3.0),
			ATFormulaToken::RightParenthesis,
		]);
	}


	#[test]
    fn test_parse_single_number() {
        let tokens = vec![ATFormulaToken::Number(3.0)];
        let result = parse(&tokens).unwrap();
        assert_eq!(result, ATFormula::Constant(3.0));
    }

	#[test]
    fn test_parse_single_parameter() {
        let tokens = vec![ATFormulaToken::Parameter("a".to_string())];
        let result = parse(&tokens).unwrap();
        assert_eq!(result, ATFormula::Parameter("a".to_string()));
    }

    #[test]
    fn test_parse_addition() {
        let tokens = vec![
            ATFormulaToken::Number(1.0),
            ATFormulaToken::Add,
            ATFormulaToken::Number(2.0),
        ];
        let result = parse(&tokens).unwrap();
        assert_eq!(
            result,
            ATFormula::Add(Box::new(ATFormula::Constant(1.0)), Box::new(ATFormula::Constant(2.0)))
        );
    }

    #[test]
    fn test_parse_complex_expression() {
        let tokens = vec![
            ATFormulaToken::Number(1.0),
            ATFormulaToken::Add,
            ATFormulaToken::LeftParenthesis,
            ATFormulaToken::Number(2.0),
            ATFormulaToken::Multiply,
            ATFormulaToken::Parameter("a".to_string()),
            ATFormulaToken::RightParenthesis,
            ATFormulaToken::Subtract,
            ATFormulaToken::Number(3.0),
        ];
        let result = parse(&tokens).unwrap();
        assert_eq!(
            result,
            ATFormula::Subtract(
                Box::new(ATFormula::Add(
                    Box::new(ATFormula::Constant(1.0)),
                    Box::new(ATFormula::Multiply(
                        Box::new(ATFormula::Constant(2.0)),
                        Box::new(ATFormula::Parameter("a".to_string()))
                    ))
                )),
                Box::new(ATFormula::Constant(3.0))
            )
        );
    }

	#[test]
	fn test_parse_formula_simple() {
		let formula = parse_formula("1 + 2").unwrap();
		assert_eq!(formula, ATFormula::Add(Box::new(ATFormula::Constant(1.0)), Box::new(ATFormula::Constant(2.0))));
	}

	#[test]
	fn test_parse_formula_complex() {
		let formula = parse_formula("1 + 2 * `param` / (3 - 4)").unwrap();
		assert_eq!(formula, ATFormula::Add(
			Box::new(ATFormula::Constant(1.0)),
			Box::new(ATFormula::Divide(
				Box::new(ATFormula::Multiply(
					Box::new(ATFormula::Constant(2.0)),
					Box::new(ATFormula::Parameter("param".to_string()))
				)),
				Box::new(ATFormula::Subtract(
					Box::new(ATFormula::Constant(3.0)),
					Box::new(ATFormula::Constant(4.0))
				))
			))
		));
	}

	#[test]
	fn test_parse_formula_nested_parentheses() {
		let formula = parse_formula("(1 + (2 * (3 - `param`)))").unwrap();
		assert_eq!(formula, ATFormula::Add(
			Box::new(ATFormula::Constant(1.0)),
			Box::new(ATFormula::Multiply(
				Box::new(ATFormula::Constant(2.0)),
				Box::new(ATFormula::Subtract(
					Box::new(ATFormula::Constant(3.0)),
					Box::new(ATFormula::Parameter("param".to_string()))
				))
			))
		));
	}

	#[test]
	fn test_parse_formula_complex_nested_unquoted() {
		let formula = parse_formula("1 + (2 * param - (3 / (4 - 5 * (param2 + 6))))").unwrap();
		assert_eq!(formula, ATFormula::Add(
			Box::new(ATFormula::Constant(1.0)),
			Box::new(ATFormula::Subtract(
				Box::new(ATFormula::Multiply(
					Box::new(ATFormula::Constant(2.0)),
					Box::new(ATFormula::Parameter("param".to_string()))
				)),
				Box::new(ATFormula::Divide(
					Box::new(ATFormula::Constant(3.0)),
					Box::new(ATFormula::Subtract(
						Box::new(ATFormula::Constant(4.0)),
						Box::new(ATFormula::Multiply(
							Box::new(ATFormula::Constant(5.0)),
							Box::new(ATFormula::Add(
								Box::new(ATFormula::Parameter("param2".to_string())),
								Box::new(ATFormula::Constant(6.0))
							))
						))
					))
				))
			))
		));
	}

	#[test]
	fn test_parse_formula_long_chain_unquoted() {
		let formula = parse_formula("1 * param + 2 / param2 - 3 * param3 + 4 / param4").unwrap();
		assert_eq!(formula, ATFormula::Add(
            Box::new(ATFormula::Subtract(
                Box::new(ATFormula::Add(
                    Box::new(ATFormula::Multiply(
                        Box::new(ATFormula::Constant(1.0)),
                        Box::new(ATFormula::Parameter("param".to_string()))
                    )),
                    Box::new(ATFormula::Divide(
                        Box::new(ATFormula::Constant(2.0)),
                        Box::new(ATFormula::Parameter("param2".to_string()))
                    )),
                )),
                Box::new(ATFormula::Multiply(
                    Box::new(ATFormula::Constant(3.0)),
                    Box::new(ATFormula::Parameter("param3".to_string()))
                )),
            )),
            Box::new(ATFormula::Divide(
                Box::new(ATFormula::Constant(4.0)),
                Box::new(ATFormula::Parameter("param4".to_string()))
            )),
        ));
	}


	#[test]
	fn test_parse_formula_unbalanced_parenthesis() {
		let result = parse_formula("(1 + 2");
		assert_eq!(result, Err(ATFormulaParsingError::UnbalancedParenthesis));
		let result = parse_formula("1 + 2)");
		assert_eq!(result, Err(ATFormulaParsingError::UnexpectedToken(Some(ATFormulaToken::RightParenthesis))));
	}

	#[test]
	fn test_parse_formula_unfinished_expression() {
		let result = parse_formula("1 + ");
		assert!(result.is_err());
	}

	#[test]
	fn test_parse_formula_malformed_expression() {
		let result = parse_formula("1 + + 2");
		assert!(result.is_err());
	}

	#[test]
	fn test_parse_formula_unfinished_parenthesized_expression() {
		let result = parse_formula("(1 + )");
		assert!(result.is_err());
	}

	#[test]
	fn test_parse_formula_malformed_parenthesized_expression() {
		let result = parse_formula("(1 + + 2)");
		assert!(result.is_err());
	}


	#[test]
	fn test_to_formula_string() {
		let formula = parse_formula("1 * param + 2 / param2 - 3 * param3 + 4 / param4").unwrap();
		assert_eq!(formula.to_formula_string(), "1 * `param` + 2 / `param2` - 3 * `param3` + 4 / `param4`");
		let formula = parse_formula("(1 + (2 * (3 - `param`)))").unwrap();
		assert_eq!(formula.to_formula_string(), "1 + 2 * (3 - `param`)");
		let formula = parse_formula("1 + 2 * `param` / (3 - 4)").unwrap();
		assert_eq!(formula.to_formula_string(), "1 + 2 * `param` / (3 - 4)");
		let formula = parse_formula("1 + (2 * param - (3 / (4 - 5 * (param2 + 6))))").unwrap();
		assert_eq!(formula.to_formula_string(), "1 + 2 * `param` - 3 / (4 - 5 * (`param2` + 6))");
	}



}