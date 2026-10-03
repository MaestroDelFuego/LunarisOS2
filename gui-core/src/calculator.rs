const DISPLAY_CAPACITY: usize = 24;
const SCALE: i64 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    fn from_key(key: char) -> Option<Self> {
        match key {
            '+' => Some(Self::Add),
            '-' => Some(Self::Subtract),
            '*' => Some(Self::Multiply),
            '/' => Some(Self::Divide),
            _ => None,
        }
    }

    const fn key(self) -> char {
        match self {
            Self::Add => '+',
            Self::Subtract => '-',
            Self::Multiply => '*',
            Self::Divide => '/',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalculationError {
    DivisionByZero,
    Overflow,
}

pub struct Calculator {
    display: [u8; DISPLAY_CAPACITY],
    display_len: usize,
    accumulator: Option<i64>,
    operation: Option<Operation>,
    replace_input: bool,
}

impl Calculator {
    pub const fn new() -> Self {
        let mut display = [0; DISPLAY_CAPACITY];
        display[0] = b'0';
        Self {
            display,
            display_len: 1,
            accumulator: None,
            operation: None,
            replace_input: false,
        }
    }

    pub fn display(&self) -> &str {
        core::str::from_utf8(&self.display[..self.display_len]).unwrap_or("0")
    }

    pub const fn pending_operation(&self) -> Option<char> {
        match self.operation {
            Some(operation) => Some(operation.key()),
            None => None,
        }
    }

    pub fn press(&mut self, key: char) -> bool {
        if matches!(key, 'c' | 'C') {
            self.clear();
            return true;
        }
        if key == '\u{8}' || key == '\u{7f}' {
            return self.backspace();
        }
        if key.is_ascii_digit() {
            return self.insert_digit(key as u8);
        }
        if key == '.' {
            return self.insert_decimal();
        }
        if let Some(operation) = Operation::from_key(key) {
            return self.select_operation(operation);
        }
        if key == '=' || key == '\r' || key == '\n' {
            return self.equals();
        }
        false
    }

    pub fn clear(&mut self) {
        self.set_display("0");
        self.accumulator = None;
        self.operation = None;
        self.replace_input = false;
    }

    fn insert_digit(&mut self, digit: u8) -> bool {
        if self.replace_input || self.is_error() {
            self.set_display("0");
            self.replace_input = false;
        }
        if self.display_len == DISPLAY_CAPACITY {
            return false;
        }
        if self.display() == "0" {
            if digit == b'0' {
                return false;
            }
            self.display[0] = digit;
            return true;
        }
        if let Some(decimal) = self.display().find('.') {
            if self.display_len - decimal - 1 == 6 {
                return false;
            }
        }
        self.display[self.display_len] = digit;
        self.display_len += 1;
        true
    }

    fn insert_decimal(&mut self) -> bool {
        if self.replace_input || self.is_error() {
            self.set_display("0");
            self.replace_input = false;
        }
        if self.display().contains('.') || self.display_len == DISPLAY_CAPACITY {
            return false;
        }
        self.display[self.display_len] = b'.';
        self.display_len += 1;
        true
    }

    fn backspace(&mut self) -> bool {
        if self.is_error() {
            self.clear();
            return true;
        }
        if self.replace_input {
            if self.operation.is_some() {
                return false;
            }
            self.replace_input = false;
        }
        if self.display_len <= 1 {
            return false;
        }
        self.display_len -= 1;
        self.display[self.display_len] = 0;
        if self.display() == "-" || self.display().is_empty() {
            self.set_display("0");
        }
        true
    }

    fn select_operation(&mut self, operation: Operation) -> bool {
        let current = match self.parse_display() {
            Some(value) => value,
            None => {
                self.set_error("OVERFLOW");
                return true;
            }
        };
        let value = match (self.operation, self.accumulator, self.replace_input) {
            (Some(previous), Some(accumulator), false) => {
                match calculate(previous, accumulator, current) {
                    Ok(result) => {
                        self.set_display_scaled(result);
                        result
                    }
                    Err(error) => {
                        self.show_error(error);
                        return true;
                    }
                }
            }
            (Some(_), Some(accumulator), true) => accumulator,
            _ => current,
        };
        self.accumulator = Some(value);
        self.operation = Some(operation);
        self.replace_input = true;
        true
    }

    fn equals(&mut self) -> bool {
        let (Some(operation), Some(accumulator)) = (self.operation, self.accumulator) else {
            return false;
        };
        if self.replace_input {
            return false;
        }
        let Some(current) = self.parse_display() else {
            self.set_error("OVERFLOW");
            return true;
        };
        match calculate(operation, accumulator, current) {
            Ok(result) => self.set_display_scaled(result),
            Err(error) => self.show_error(error),
        }
        self.accumulator = None;
        self.operation = None;
        self.replace_input = true;
        true
    }

    fn parse_display(&self) -> Option<i64> {
        let bytes = self.display().as_bytes();
        let negative = bytes.first() == Some(&b'-');
        let digits = if negative { &bytes[1..] } else { bytes };
        let decimal = digits.iter().position(|byte| *byte == b'.');
        let whole_digits = decimal.map_or(digits, |index| &digits[..index]);
        let fractional_digits = decimal.map_or(&[][..], |index| &digits[index + 1..]);
        if fractional_digits.len() > 6 {
            return None;
        }
        let mut whole = 0i128;
        for digit in whole_digits {
            if !digit.is_ascii_digit() {
                return None;
            }
            whole = whole
                .checked_mul(10)?
                .checked_add(i128::from(digit - b'0'))?;
        }
        let mut fraction = 0i128;
        for digit in fractional_digits {
            if !digit.is_ascii_digit() {
                return None;
            }
            fraction = fraction
                .checked_mul(10)?
                .checked_add(i128::from(digit - b'0'))?;
        }
        for _ in fractional_digits.len()..6 {
            fraction *= 10;
        }
        let scaled = whole
            .checked_mul(i128::from(SCALE))?
            .checked_add(fraction)?;
        let signed = if negative { -scaled } else { scaled };
        i64::try_from(signed).ok()
    }

    fn set_display_scaled(&mut self, value: i64) {
        let mut output = [0; DISPLAY_CAPACITY];
        let mut length = 0;
        let magnitude = i128::from(value).unsigned_abs();
        if value < 0 {
            output[length] = b'-';
            length += 1;
        }

        let whole = magnitude / SCALE as u128;
        let mut reversed = [0; 20];
        let mut whole_length = 0;
        let mut remaining = whole;
        loop {
            reversed[whole_length] = b'0' + (remaining % 10) as u8;
            whole_length += 1;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        for digit in reversed[..whole_length].iter().rev() {
            output[length] = *digit;
            length += 1;
        }

        let fraction = (magnitude % SCALE as u128) as u32;
        if fraction != 0 {
            let mut fractional_digits = [0; 6];
            let mut remainder = fraction;
            for digit in fractional_digits.iter_mut().rev() {
                *digit = b'0' + (remainder % 10) as u8;
                remainder /= 10;
            }
            let fractional_length = fractional_digits
                .iter()
                .rposition(|digit| *digit != b'0')
                .map_or(0, |last| last + 1);
            output[length] = b'.';
            length += 1;
            output[length..length + fractional_length]
                .copy_from_slice(&fractional_digits[..fractional_length]);
            length += fractional_length;
        }
        self.display[..length].copy_from_slice(&output[..length]);
        self.display[length..].fill(0);
        self.display_len = length;
    }

    fn set_display(&mut self, value: &str) {
        self.display.fill(0);
        self.display_len = value.len().min(DISPLAY_CAPACITY);
        self.display[..self.display_len].copy_from_slice(&value.as_bytes()[..self.display_len]);
    }

    fn set_error(&mut self, message: &str) {
        self.set_display(message);
        self.accumulator = None;
        self.operation = None;
        self.replace_input = true;
    }

    fn show_error(&mut self, error: CalculationError) {
        self.set_error(match error {
            CalculationError::DivisionByZero => "DIV/0",
            CalculationError::Overflow => "OVERFLOW",
        });
    }

    fn is_error(&self) -> bool {
        self.display() == "DIV/0" || self.display() == "OVERFLOW"
    }
}

fn calculate(operation: Operation, left: i64, right: i64) -> Result<i64, CalculationError> {
    match operation {
        Operation::Add => left.checked_add(right).ok_or(CalculationError::Overflow),
        Operation::Subtract => left.checked_sub(right).ok_or(CalculationError::Overflow),
        Operation::Multiply => {
            let product = i128::from(left) * i128::from(right) / i128::from(SCALE);
            i64::try_from(product).map_err(|_| CalculationError::Overflow)
        }
        Operation::Divide if right == 0 => Err(CalculationError::DivisionByZero),
        Operation::Divide => {
            let quotient = i128::from(left) * i128::from(SCALE) / i128::from(right);
            i64::try_from(quotient).map_err(|_| CalculationError::Overflow)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Calculator;

    fn enter(calculator: &mut Calculator, keys: &str) {
        for key in keys.chars() {
            calculator.press(key);
        }
    }

    #[test]
    fn adds_and_multiplies_decimal_values() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "12.5+0.5=");
        assert_eq!(calculator.display(), "13");

        enter(&mut calculator, "2.5*4=");
        assert_eq!(calculator.display(), "10");
    }

    #[test]
    fn chains_operations_from_left_to_right() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "2+3*4=");
        assert_eq!(calculator.display(), "20");
    }

    #[test]
    fn subtraction_formats_negative_results() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "3-5=");
        assert_eq!(calculator.display(), "-2");
    }

    #[test]
    fn division_is_truncated_to_six_decimal_places() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "1/3=");
        assert_eq!(calculator.display(), "0.333333");
    }

    #[test]
    fn reports_division_by_zero_and_recovers_on_digit_input() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "7/0=");
        assert_eq!(calculator.display(), "DIV/0");
        enter(&mut calculator, "4");
        assert_eq!(calculator.display(), "4");
    }

    #[test]
    fn backspace_and_clear_edit_the_current_entry() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "123.4");
        assert!(calculator.press('\u{8}'));
        assert_eq!(calculator.display(), "123.");
        calculator.clear();
        assert_eq!(calculator.display(), "0");
        assert_eq!(calculator.pending_operation(), None);
    }

    #[test]
    fn accepts_no_more_than_six_fractional_digits() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "0.123456");
        assert!(!calculator.press('7'));
        assert_eq!(calculator.display(), "0.123456");
    }

    #[test]
    fn checked_arithmetic_reports_overflow() {
        let mut calculator = Calculator::new();
        enter(&mut calculator, "9000000000000+9000000000000=");
        assert_eq!(calculator.display(), "OVERFLOW");
    }
}
