#[derive(Clone)]
pub struct Place {
    pub id: i64,
    pub name: String,
    pub note: String,
}
impl Place {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.note]
    }
}

#[derive(Clone)]
pub struct Exercise {
    pub id: i64,
    pub name: String,
    pub note: String,
}
impl Exercise {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.note]
    }
}

#[derive(Clone)]
pub struct Food {
    pub id: i64,
    pub name: String,
    pub calories: f64,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbohydrate: Option<f64>,
    pub note: String,
}
impl Food {
    pub fn to_line(&self) -> [String; 6] {
        [
            self.name.to_owned(),
            format!("{:.1}", self.calories),
            format!("{:.1}", self.protein.unwrap_or_default()),
            format!("{:.1}", self.fat.unwrap_or_default()),
            format!("{:.1}", self.carbohydrate.unwrap_or_default()),
            self.note.to_owned(),
        ]
    }
}

#[derive(Clone)]
pub struct HistoryItem {
    pub date: String,
    pub load: f64,
    pub rep: f64,
    pub note: String,
}

#[derive(Clone, Default)]
pub struct Totals {
    pub calories: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
}
impl Totals {
    pub fn to_lines_today(&self) -> [[String; 4]; 2] {
        [
            [
                format!("calories kcal"),
                format!("protein g"),
                format!("fat g"),
                format!("carbohydrate g"),
            ],
            [
                format!("{:.2}", self.calories),
                format!("{:.2}", self.protein),
                format!("{:.2}", self.fat),
                format!("{:.2}", self.carbohydrate),
            ],
        ]
    }
}
