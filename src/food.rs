use crate::com::*;

#[derive(Args)]
#[args(desc = "Currency settings.")]
pub struct Main {}
impl Run<C> for Main {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(food(c, a)?)
    }
}
pub fn print_food(e: &Food) -> [&str; 2] {
    [&e.name, &e.desc]
}
pub async fn reg(db: &mut Db) -> Res<i64> {
    let foods = db.foods().await?;

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
            foods: foods.iter().map(|e| e.name.to_owned()).collect(),
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
    Ok(db
        .new_food(&name, calories, protein, fat, carbohydrate, &desc)
        .await?)
}
#[tokio::main]
async fn food(c: &C, _a: Main) -> Res<()> {
    let mut db = c.db().await?;
    loop {
        let curs = db.foods().await?;
        println!("{}", to_table(&curs.iter().map(print_food).collect_vec()));

        #[derive(Actions, Clone)]
        enum A {
            Add,
            Sel,
            Exit,
        }

        use A::*;
        match A::get("Choose action", None)? {
            Add => {
                reg(&mut db).await?;
            }
            Sel => {
                let s = &curs[select_line("Choose food", &curs, |e| {
                    print_food(e).map(|e| e.to_owned())
                })
                .prompt()?
                .index];

                #[derive(Clone, Actions)]
                enum B {
                    Del,
                    Upd,
                }

                use B::*;
                match B::get("Choose action", None)? {
                    Del => {
                        if Confirm::new("Are you sure?").prompt()? {
                            db.del_food(s.id).await?;
                        }
                    }
                    Upd => {
                        db.upd_food(
                            s.id,
                            &Text::new("Name").with_initial_value(&s.name).prompt()?,
                            CustomType::<f64>::new("calories")
                                .with_starting_input(&format!("{:.2}", s.calories))
                                .prompt()?,
                            CustomType::<f64>::new("protein")
                                .with_starting_input(
                                    &s.protein
                                        .map(|p| format!("{:.2}", p))
                                        .unwrap_or(format!("")),
                                )
                                .with_help_message("Press ESC if unknown")
                                .prompt_skippable()?,
                            CustomType::<f64>::new("fat")
                                .with_starting_input(
                                    &s.fat.map(|p| format!("{:.2}", p)).unwrap_or(format!("")),
                                )
                                .with_help_message("Press ESC if unknown")
                                .prompt_skippable()?,
                            CustomType::<f64>::new("carbohydrate")
                                .with_starting_input(
                                    &s.carbohydrate
                                        .map(|p| format!("{:.2}", p))
                                        .unwrap_or(format!("")),
                                )
                                .with_help_message("Press ESC if unknown")
                                .prompt_skippable()?,
                            &Text::new("Desc").with_initial_value(&s.desc).prompt()?,
                        )
                        .await?;
                    }
                }
            },
            Exit => break,
        }
    }

    Ok(())
}
