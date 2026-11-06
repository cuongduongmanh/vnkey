//! Bộ xử lý Telex -> Unicode dựng sẵn (NFC).
//!
//! Engine chỉ làm việc với từng *từ* đang gõ. Mỗi phím chữ cái được đưa vào
//! [`Engine::process`], engine trả về [`Action`] cho biết nên để phím đi qua
//! hay xoá `backspaces` ký tự rồi gõ lại `text`. Mọi phím không phải chữ cái
//! (dấu cách, dấu câu, Enter, mũi tên...) kết thúc từ hiện tại.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mark {
    None,
    /// â ê ô
    Hat,
    /// ă
    Breve,
    /// ơ ư
    Horn,
    /// đ
    Stroke,
}

#[derive(Clone, Copy, Debug)]
struct Letter {
    /// Chữ cái ASCII thường (a-z)
    base: char,
    upper: bool,
    mark: Mark,
}

impl Letter {
    fn plain(c: char) -> Self {
        Letter { base: c.to_ascii_lowercase(), upper: c.is_ascii_uppercase(), mark: Mark::None }
    }
}

/// Việc mà tầng hệ điều hành cần làm sau mỗi phím.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Để phím gốc đi qua bình thường.
    PassThrough,
    /// Chặn phím gốc, gửi `backspaces` lần Backspace rồi gõ `text`.
    Replace { backspaces: usize, text: String },
}

/// Thứ tự cột: không dấu, huyền (f), sắc (s), hỏi (r), ngã (x), nặng (j)
const TONE_ROWS: [&str; 12] = [
    "aàáảãạ", "ăằắẳẵặ", "âầấẩẫậ", "eèéẻẽẹ", "êềếểễệ", "iìíỉĩị",
    "oòóỏõọ", "ôồốổỗộ", "ơờớởỡợ", "uùúủũụ", "ưừứửữự", "yỳýỷỹỵ",
];
const ROW_INFO: [(char, Mark); 12] = [
    ('a', Mark::None), ('a', Mark::Breve), ('a', Mark::Hat), ('e', Mark::None),
    ('e', Mark::Hat), ('i', Mark::None), ('o', Mark::None), ('o', Mark::Hat),
    ('o', Mark::Horn), ('u', Mark::None), ('u', Mark::Horn), ('y', Mark::None),
];
const TONE_KEYS: [char; 6] = ['_', 'f', 's', 'r', 'x', 'j'];

/// Phụ âm đầu hợp lệ (đã gồm các tiền tố khi đang gõ dở như "n", "ng").
const INITIALS: [&str; 28] = [
    "", "b", "c", "ch", "d", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh",
    "nh", "p", "ph", "q", "qu", "r", "s", "t", "th", "tr", "v", "x",
];
/// Phụ âm cuối hợp lệ.
const FINALS: [&str; 9] = ["", "c", "ch", "m", "n", "ng", "nh", "p", "t"];
/// Nguyên âm / cụm nguyên âm hợp lệ (đã gồm các tiền tố khi gõ dở).
const VOWEL_CLUSTERS: [&str; 55] = [
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y",
    "ai", "ao", "au", "ay", "âu", "ây", "eo", "êu", "ia", "iê", "iu", "oa", "oă", "oe",
    "oi", "oo", "ôi", "ơi", "ua", "uâ", "uê", "ui", "uô", "uơ", "ưa", "ưi", "ưu", "ươ",
    "uy", "yê",
    "iêu", "oai", "oao", "oay", "oeo", "uây", "uôi", "ươi", "ươu", "uya", "uyê", "uyu", "yêu",
];

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

fn compose(base: char, mark: Mark, tone: u8) -> char {
    if base == 'd' && mark == Mark::Stroke {
        return 'đ';
    }
    match ROW_INFO.iter().position(|&(b, m)| b == base && m == mark) {
        Some(row) => TONE_ROWS[row].chars().nth(tone as usize).unwrap_or(base),
        None => base,
    }
}

/// Tách một ký tự đang hiển thị thành (chữ cái, dấu thanh).
fn decompose(c: char) -> Option<(Letter, u8)> {
    let lower = c.to_lowercase().next()?;
    let upper = lower != c;
    if lower == 'đ' {
        return Some((Letter { base: 'd', upper, mark: Mark::Stroke }, 0));
    }
    if lower.is_ascii_lowercase() {
        return Some((Letter { base: lower, upper, mark: Mark::None }, 0));
    }
    for (row, chars) in TONE_ROWS.iter().enumerate() {
        if let Some(tone) = chars.chars().position(|x| x == lower) {
            let (base, mark) = ROW_INFO[row];
            return Some((Letter { base, upper, mark }, tone as u8));
        }
    }
    None
}

pub struct Engine {
    letters: Vec<Letter>,
    /// Các phím đã gõ cho từ hiện tại, dùng để trả lại nguyên văn khi từ không phải tiếng Việt.
    keys: Vec<char>,
    /// 0 = không dấu, 1..=5 = f s r x j
    tone: u8,
    /// Từ đã bị "trả về nguyên văn" (gõ lặp để huỷ, hoặc không phải tiếng Việt).
    raw: bool,
    /// Vị trí chữ ư sinh ra từ phím `w` đứng riêng (để `ww` -> `w`).
    w_idx: Option<usize>,
    /// Những gì đang hiện trên màn hình cho từ này.
    shown: Vec<char>,
    /// `true`: đặt dấu kiểu mới (hoà, thuý). `false`: kiểu cũ (hòa, thúy).
    pub modern_tone: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            letters: Vec::new(),
            keys: Vec::new(),
            tone: 0,
            raw: false,
            w_idx: None,
            shown: Vec::new(),
            modern_tone: false,
        }
    }

    /// Bắt đầu từ mới (gọi khi gặp dấu cách, click chuột, đổi ứng dụng...).
    pub fn reset(&mut self) {
        self.letters.clear();
        self.keys.clear();
        self.tone = 0;
        self.raw = false;
        self.w_idx = None;
        self.shown.clear();
    }

    /// Xử lý một phím ký tự.
    pub fn process(&mut self, ch: char) -> Action {
        if !ch.is_ascii_alphabetic() {
            self.reset();
            return Action::PassThrough;
        }
        self.keys.push(ch);
        let consumed = !self.raw && self.apply(ch);
        if !consumed {
            self.letters.push(Letter::plain(ch));
        }
        if !self.raw && !self.is_valid() && self.is_transformed() {
            self.restore();
        }
        self.sync(ch)
    }

    /// Người dùng nhấn Backspace. Phím luôn được để đi qua; engine chỉ cập nhật trạng thái.
    pub fn backspace(&mut self) -> Action {
        if self.shown.pop().is_none() {
            self.reset();
            return Action::PassThrough;
        }
        let shown = std::mem::take(&mut self.shown);
        self.reset();
        for &c in &shown {
            match decompose(c) {
                Some((letter, tone)) => {
                    self.letters.push(letter);
                    if tone != 0 {
                        self.tone = tone;
                    }
                }
                None => {
                    self.reset();
                    return Action::PassThrough;
                }
            }
        }
        self.keys = self.to_telex();
        self.shown = shown;
        Action::PassThrough
    }

    // ---------------------------------------------------------------- biến đổi

    fn apply(&mut self, ch: char) -> bool {
        match ch.to_ascii_lowercase() {
            'f' => self.apply_tone(1, ch),
            's' => self.apply_tone(2, ch),
            'r' => self.apply_tone(3, ch),
            'x' => self.apply_tone(4, ch),
            'j' => self.apply_tone(5, ch),
            'z' => {
                if self.tone != 0 {
                    self.tone = 0;
                    true
                } else {
                    false
                }
            }
            c @ ('a' | 'e' | 'o') => self.apply_hat(c, ch),
            'w' => self.apply_w(ch),
            'd' => self.apply_stroke(ch),
            _ => false,
        }
    }

    /// Phím biến đổi bị gõ lặp: huỷ biến đổi và in ra phím nguyên văn.
    fn undo_literal(&mut self, ch: char) {
        self.letters.push(Letter::plain(ch));
        self.raw = true;
    }

    fn apply_tone(&mut self, tone: u8, ch: char) -> bool {
        if self.vowel_range().is_none() || !self.is_valid() {
            return false;
        }
        if self.tone == tone {
            self.tone = 0;
            self.undo_literal(ch);
        } else {
            self.tone = tone;
        }
        true
    }

    fn apply_hat(&mut self, base: char, ch: char) -> bool {
        let Some((s, e)) = self.vowel_range() else { return false };
        if !self.is_valid() {
            return false;
        }
        let Some(i) = (s..e).rev().find(|&i| self.letters[i].base == base) else { return false };
        if self.letters[i].mark == Mark::Hat {
            self.letters[i].mark = Mark::None;
            self.undo_literal(ch);
        } else {
            self.letters[i].mark = Mark::Hat;
        }
        true
    }

    fn apply_w(&mut self, ch: char) -> bool {
        if !self.is_valid() {
            return false;
        }
        let Some((s, e)) = self.vowel_range() else {
            // `w` đứng riêng sau phụ âm -> ư
            self.letters.push(Letter { base: 'u', upper: ch.is_ascii_uppercase(), mark: Mark::Horn });
            self.w_idx = Some(self.letters.len() - 1);
            return true;
        };
        // `ww` -> `w`
        let last = self.letters.len() - 1;
        if self.w_idx == Some(last) && self.letters[last].mark == Mark::Horn {
            self.letters[last] = Letter { base: 'w', upper: self.letters[last].upper, mark: Mark::None };
            self.w_idx = None;
            self.raw = true;
            return true;
        }
        // uo + w -> ươ
        if let Some(i) = (s..e.saturating_sub(1))
            .find(|&i| self.letters[i].base == 'u' && self.letters[i + 1].base == 'o')
        {
            if self.letters[i].mark == Mark::Horn && self.letters[i + 1].mark == Mark::Horn {
                self.letters[i].mark = Mark::None;
                self.letters[i + 1].mark = Mark::None;
                self.undo_literal(ch);
            } else {
                self.letters[i].mark = Mark::Horn;
                self.letters[i + 1].mark = Mark::Horn;
            }
            return true;
        }
        // a -> ă, o -> ơ, u -> ư
        let Some(i) = (s..e).rev().find(|&i| matches!(self.letters[i].base, 'a' | 'o' | 'u')) else {
            return false;
        };
        let target = if self.letters[i].base == 'a' { Mark::Breve } else { Mark::Horn };
        if self.letters[i].mark == target {
            self.letters[i].mark = Mark::None;
            self.undo_literal(ch);
        } else {
            self.letters[i].mark = target;
        }
        true
    }

    fn apply_stroke(&mut self, ch: char) -> bool {
        if self.letters.first().map(|l| l.base) != Some('d') || !self.is_valid() {
            return false;
        }
        if self.letters[0].mark == Mark::Stroke {
            self.letters[0].mark = Mark::None;
            self.undo_literal(ch);
        } else {
            self.letters[0].mark = Mark::Stroke;
        }
        true
    }

    // ---------------------------------------------------------------- phân tích từ

    /// Vị trí [start, end) của cụm nguyên âm chính. "qu" và "gi" được tính là phụ âm đầu.
    fn vowel_range(&self) -> Option<(usize, usize)> {
        let first = self.letters.iter().position(|l| is_vowel(l.base))?;
        let mut end = first;
        while end < self.letters.len() && is_vowel(self.letters[end].base) {
            end += 1;
        }
        let mut start = first;
        if end - start > 1 && start > 0 {
            let prev = self.letters[start - 1].base;
            let cur = self.letters[start].base;
            if (prev == 'q' && cur == 'u') || (prev == 'g' && cur == 'i') {
                start += 1;
            }
        }
        Some((start, end))
    }

    /// Từ hiện tại có còn là (tiền tố của) một âm tiết tiếng Việt không.
    fn is_valid(&self) -> bool {
        let bases: String = self.letters.iter().map(|l| l.base).collect();
        match self.vowel_range() {
            None => INITIALS.contains(&bases.as_str()),
            Some((s, e)) => {
                INITIALS.contains(&&bases[..s])
                    && FINALS.contains(&&bases[e..])
                    && self.cluster_valid(&self.letters[s..e])
            }
        }
    }

    /// Cụm nguyên âm hợp lệ nếu khớp một cụm chuẩn, cho phép chữ chưa gõ dấu mũ/móc.
    fn cluster_valid(&self, cluster: &[Letter]) -> bool {
        VOWEL_CLUSTERS.iter().any(|pattern| {
            let pat: Vec<Letter> = pattern.chars().filter_map(|c| decompose(c).map(|d| d.0)).collect();
            pat.len() == cluster.len()
                && pat.iter().zip(cluster).all(|(p, c)| {
                    p.base == c.base && (c.mark == Mark::None || c.mark == p.mark)
                })
        })
    }

    fn is_transformed(&self) -> bool {
        self.tone != 0
            || self.letters.len() != self.keys.len()
            || self.letters.iter().any(|l| l.mark != Mark::None)
    }

    /// Trả lại nguyên văn các phím đã gõ (vd: gõ tiếng Anh "what").
    fn restore(&mut self) {
        self.letters = self.keys.iter().map(|&c| Letter::plain(c)).collect();
        self.tone = 0;
        self.w_idx = None;
        self.raw = true;
    }

    /// Chữ nào trong từ mang dấu thanh.
    fn tone_position(&self) -> Option<usize> {
        let (s, e) = self.vowel_range()?;
        // Ưu tiên nguyên âm có mũ/móc cuối cùng: tiếng, người, khuyển
        if let Some(i) = (s..e).rev().find(|&i| matches!(self.letters[i].mark, Mark::Hat | Mark::Breve | Mark::Horn)) {
            return Some(i);
        }
        Some(match e - s {
            1 => s,
            2 => {
                let has_final = e < self.letters.len();
                if has_final {
                    s + 1 // toán, hoàng
                } else {
                    let pair = (self.letters[s].base, self.letters[s + 1].base);
                    if matches!(pair, ('o', 'a') | ('o', 'e') | ('u', 'y')) && self.modern_tone {
                        s + 1 // hoà, thuý
                    } else {
                        s // hòa, mái, mùa
                    }
                }
            }
            _ => s + 1, // ngoài, khuỷu
        })
    }

    fn render(&self) -> Vec<char> {
        let tone_pos = if self.tone != 0 { self.tone_position() } else { None };
        self.letters
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let tone = if Some(i) == tone_pos { self.tone } else { 0 };
                let c = compose(l.base, l.mark, tone);
                if l.upper { c.to_uppercase().next().unwrap_or(c) } else { c }
            })
            .collect()
    }

    fn to_telex(&self) -> Vec<char> {
        let mut out = Vec::new();
        for l in &self.letters {
            out.push(if l.upper { l.base.to_ascii_uppercase() } else { l.base });
            match l.mark {
                Mark::Hat => out.push(l.base),
                Mark::Breve | Mark::Horn => out.push('w'),
                Mark::Stroke => out.push('d'),
                Mark::None => {}
            }
        }
        if self.tone != 0 {
            out.push(TONE_KEYS[self.tone as usize]);
        }
        out
    }

    /// So sánh với những gì đang hiển thị để sinh ra Action tối thiểu.
    fn sync(&mut self, typed: char) -> Action {
        let new = self.render();
        let action = if new.len() == self.shown.len() + 1
            && new.starts_with(&self.shown)
            && new.last() == Some(&typed)
        {
            Action::PassThrough
        } else {
            let prefix = self.shown.iter().zip(&new).take_while(|(a, b)| a == b).count();
            Action::Replace {
                backspaces: self.shown.len() - prefix,
                text: new[prefix..].iter().collect(),
            }
        };
        self.shown = new;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Giả lập gõ phím vào một ô văn bản. `<` = Backspace.
    fn typ_with(mut e: Engine, input: &str) -> String {
        let mut screen: Vec<char> = Vec::new();
        for ch in input.chars() {
            if ch == '<' {
                e.backspace();
                screen.pop();
                continue;
            }
            match e.process(ch) {
                Action::PassThrough => screen.push(ch),
                Action::Replace { backspaces, text } => {
                    for _ in 0..backspaces {
                        screen.pop();
                    }
                    screen.extend(text.chars());
                }
            }
        }
        screen.into_iter().collect()
    }

    fn typ(input: &str) -> String {
        typ_with(Engine::new(), input)
    }

    #[test]
    fn basic_words() {
        let cases = [
            ("vieejt nam", "việt nam"),
            ("tieengs", "tiếng"),
            ("nguoiwf", "người"),
            ("dduwowcj", "được"),
            ("duwowngf", "dường"),
            ("ddoongf", "đồng"),
            ("toans", "toán"),
            ("hoangf", "hoàng"),
            ("quys", "quý"),
            ("gias", "giá"),
            ("gif", "gì"),
            ("giuwax", "giữa"),
            ("khuyeenr", "khuyển"),
            ("chuyeenj", "chuyện"),
            ("Nguyeenx", "Nguyễn"),
            ("mais", "mái"),
            ("muaf", "mùa"),
            ("cuar", "của"),
            ("tuaanf", "tuần"),
            ("tranaf", "trần"),
            ("ruouwj", "rượu"),
            ("ngoaif", "ngoài"),
            ("xoawn", "xoăn"),
        ];
        for (input, want) in cases {
            assert_eq!(typ(input), want, "input: {input}");
        }
    }

    #[test]
    fn tone_style() {
        assert_eq!(typ("hoaf"), "hòa");
        assert_eq!(typ("Thuyr"), "Thủy");
        let mut e = Engine::new();
        e.modern_tone = true;
        assert_eq!(typ_with(e, "hoaf"), "hoà");
    }

    #[test]
    fn uppercase() {
        assert_eq!(typ("DDaay"), "Đây");
        assert_eq!(typ("VIEEJT"), "VIỆT");
        assert_eq!(typ("Vieejt"), "Việt");
    }

    #[test]
    fn standalone_w() {
        assert_eq!(typ("w"), "ư");
        assert_eq!(typ("tw"), "tư");
        assert_eq!(typ("ww"), "w");
        assert_eq!(typ("wf"), "ừ");
    }

    #[test]
    fn double_key_undo() {
        assert_eq!(typ("ass"), "as");
        assert_eq!(typ("aaa"), "aa");
        assert_eq!(typ("ddd"), "dd");
        assert_eq!(typ("aww"), "aw");
        assert_eq!(typ("asf"), "à");
        assert_eq!(typ("asz"), "a");
        assert_eq!(typ("az"), "az");
    }

    #[test]
    fn english_words_restored() {
        assert_eq!(typ("what"), "what");
        assert_eq!(typ("were"), "were");
        assert_eq!(typ("windows"), "windows");
    }

    #[test]
    fn backspace() {
        assert_eq!(typ("mas<"), "m");
        assert_eq!(typ("mas<a"), "ma");
        assert_eq!(typ("toans<"), "toá");
        assert_eq!(typ("toans<n"), "toán");
        assert_eq!(typ("tieeng<<s"), "tiế");
    }

    #[test]
    fn minimal_replace() {
        let mut e = Engine::new();
        assert_eq!(e.process('v'), Action::PassThrough);
        assert_eq!(e.process('i'), Action::PassThrough);
        assert_eq!(e.process('e'), Action::PassThrough);
        assert_eq!(e.process('e'), Action::Replace { backspaces: 1, text: "ê".into() });
        assert_eq!(e.process('j'), Action::Replace { backspaces: 1, text: "ệ".into() });
        assert_eq!(e.process('t'), Action::PassThrough);
    }
}
