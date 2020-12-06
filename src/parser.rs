use std::vec::IntoIter;
use std::iter::Peekable;

pub trait GentleIterator<I: Iterator> {
    fn take_until<F>(&mut self, predicate: F) -> IntoIter<I::Item>
        where F: Fn(&I::Item) -> bool;
}

impl<I: Iterator> GentleIterator<I> for Peekable<I> {
    fn take_until<F>(&mut self, predicate: F) -> IntoIter<I::Item>
        where F: Fn(&I::Item) -> bool {

        let mut v: Vec<I::Item> = vec![];
        while self.peek().map_or(false, &predicate) {
            v.push(self.next().unwrap());
        }

        v.into_iter()
    }
}

pub fn check<F,I>(iter: &mut Peekable<I>, fun: F) -> bool
where F: Fn(char) -> bool,
      I: Iterator<Item = char> {
    if let Some(&x) = iter.peek() {
        fun(x)
    } else {
        false
    }
}

pub fn check_chr<I>(iter: &mut Peekable<I>, chr: char) -> bool
where I: Iterator<Item = char> {
    check(iter, |x| x == chr)
}

pub fn parse_around<I>(iter: &mut Peekable<I>, beg: char, end: char) -> Option<String>
where I: Iterator<Item = char> {
    if !check_chr(iter, beg) {
        return None
    }

    iter.next(); // Consume beg
    let value = iter.take_until(|c| *c != end).collect();
    iter.next(); // Consume end
    parse_whitespace(iter);

    Some(value)
}

pub fn parse_prefixed<I>(iter: &mut Peekable<I>, chr: char) -> Option<String>
where I: Iterator<Item = char> {
    if !check_chr(iter, chr) {
        return None
    }

    iter.next(); // Consume the opening chr
    let value = iter.take_until(|c| *c != ' ').collect();
    parse_whitespace(iter);

    Some(value)
}

pub fn parse_plain<I>(iter: &mut Peekable<I>) -> Option<String>
where I: Iterator<Item = char> {
    if let Some(&x) = iter.peek() {
        let value = iter.take_until(|c| *c != ' ').collect();
        parse_whitespace(iter);

        return Some(value)
    }

    None
}

pub fn parse_whitespace<I>(iter: &mut Peekable<I>) -> bool
where I: Iterator<Item = char> {
    if check_chr(iter, ' ') || check_chr(iter, '\n') || check_chr(iter, '\t') {
        iter.next();
        true
    } else {
        false
    }
}
