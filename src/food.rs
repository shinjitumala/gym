use crate::com::*;

pub fn reg(db: &mut Db) -> Res<usize> {
    let foods = db.foods()?;

    #[derive(Clone)]
    struct V {
        foods: Vec<String>,
    }
    impl StringValidator for V {
        fn validate(
            &self,
            input: &str,
        ) -> Result<inquire::validator::Validation, inquire::CustomUserError> {
            let b = self.foods.contains(&input.to_string());
            Ok(if b {
                Validation::Invalid(format!("Food name already taken").into())
            } else {
                Validation::Valid
            })
        }
    }
    let name = Text::new("Food name")
        .with_validator(V {
            foods: foods.iter().map(|(_, e)| e.name.to_owned()).collect(),
        })
        .prompt()?;
    let calories = CustomType::<f64>::new("calories").prompt()?;
    let protein = CustomType::<f64>::new("protein")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let fat = CustomType::<f64>::new("fat")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let carbohydrate = CustomType::<f64>::new("carbohydrate")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let desc = Text::new("desc").prompt()?;
    Ok(db.new_food(&name, calories, protein, fat, carbohydrate, &desc)?)
}
