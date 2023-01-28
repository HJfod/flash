
use std::ops::Range;

struct Annotation {
    raw: String,
    range: Range<usize>,
    value: Option<String>,
}

pub struct Annotations<'a> {
    next_in_iter: usize,
    raw: &'a str,
    /// These better be in sorted order by range or shit will break bad!
    annotations: Vec<Annotation>,
}

impl<'a> Annotations<'a> {
    pub fn new(raw: &'a str) -> Self {
        Self {
            raw,
            next_in_iter: 0,
            annotations: Self::create_annotations(raw),
        }
    }

    pub fn into_result(self) -> String {
        let mut result = String::from(self.raw);
        let mut offset = 0isize;
        for word in self.annotations {
            if let Some(value) = word.value {
                result.replace_range(
                    (word.range.start as isize + offset) as usize
                    ..(word.range.end as isize + offset) as usize,
                    &value
                );
                // Applying this annotation may cause the next annotations to 
                // shifted if the replaced string is shorter / longer than the 
                // original
                offset += value.len() as isize - word.raw.len() as isize;
            }
        }
        result
    }

    pub fn rewind(&mut self) {
        self.next_in_iter = 0;
    }

    pub fn next(&mut self) -> Option<String> {
        self.annotations.iter()
            .skip(self.next_in_iter)
            .skip_while(|a| {
                if a.value.is_some() {
                    self.next_in_iter += 1;
                    true
                }
                else {
                    false
                }
            })
            .next()
            .inspect(|_| self.next_in_iter += 1)
            .map(|a| a.raw.clone())
    }

    pub fn annotate(&mut self, value: String) {
        self.annotations.get_mut(self.next_in_iter - 1).unwrap().value = Some(value);
    }

    fn skip_to_next_word(raw: &'a str, iter_ix: &mut usize) {
        while let Some(i) = raw.chars().nth(*iter_ix) && !i.is_alphanumeric() {
            *iter_ix += 1;
        }
    }

    fn next_word(raw: &'a str, iter_ix: &mut usize) -> Option<(Range<usize>, String)> {
        let start = *iter_ix;
        let res: String = raw.chars()
            .skip(*iter_ix)
            .take_while(|c| c.is_alphanumeric())
            .collect();
        *iter_ix += res.len();
        let end = *iter_ix;
        (!res.is_empty()).then_some((start..end, res))
    }

    fn next_annotation(raw: &'a str, iter_ix: &mut usize) -> Option<Annotation> {
        Self::skip_to_next_word(raw, iter_ix);
        let word = Self::next_word(raw, iter_ix)?;
        let (range, word) = word;
        Some(Annotation {
            raw: word.clone(),
            range,
            value: None
        })
    }

    fn create_annotations(raw: &'a str) -> Vec<Annotation> {
        let mut res = Vec::new();
        let mut iter_ix = 0;
        while let Some(a) = Self::next_annotation(raw, &mut iter_ix) {
            res.push(a);
        }
        res
    }
}
