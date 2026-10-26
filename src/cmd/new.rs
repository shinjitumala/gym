use crate::com::*;

#[derive(Args)]
#[args(f = new_session, desc = "New session at the gym to the buffer.")]
pub struct New {}
fn new_session(c: &C, _a: New) -> Res<()> {
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    let (pid, _) = input_place(c, &d)?;
    let date = input_date2("Training time")?;
    let desc = Text::new("Description")
        .prompt_skippable()?
        .unwrap_or_default();
    let sid = c.block_on(d.new_session(pid, &date.to_string(), &desc))?;

    loop {
        let ecomp = TextWithAutocomplete::new(
            c.block_on(d.exercises(pid))?
                .into_iter()
                .map(|e| (e.id, e))
                .collect(),
            |(_, e)| [e.name.to_owned(), e.note.to_owned()],
        );

        let e = Text::new("Exercise")
            .with_autocomplete(ecomp.clone())
            .with_help_message("Press ESC if done")
            .prompt_skippable()?
            .map(|e| e.trim().to_owned());

        let Some(e) = e else {
            break;
        };
        let exercise = c.block_on(d.get_or_create_exercise(&e))?;
        print_history(&c.block_on(d.history(pid, exercise.id, 3))?)?;

        loop {
            let Some(load) = CustomType::<f64>::new("load")
                .with_help_message("Press ESC if done with exercise")
                .prompt_skippable()?
            else {
                break;
            };
            loop {
                let Some(rep) = CustomType::<f64>::new("rep")
                    .with_help_message("Press ESC if done with load")
                    .prompt_skippable()?
                else {
                    break;
                };
                let note = Text::new("Notes").prompt()?;
                c.block_on(d.new_set(sid, exercise.id, &t2s(Utc::now()), load, rep, &note))?;
            }
        }
    }
    Ok(())
}

fn input_place(c: &C, d: &crate::db::Db) -> Res<(i64, crate::model::Place)> {
    let places = c.block_on(d.places())?;
    let lines = to_lines(&places.iter().map(|e| e.to_line()).collect_vec())
        .into_iter()
        .enumerate()
        .map(|(i, e)| ListOption::new(i, e))
        .collect();
    let i = Select::new("Place", lines).prompt()?.index;
    let p = places
        .into_iter()
        .nth(i)
        .ok_or_else(|| "No place to choose.".to_string())?;
    Ok((p.id, p))
}

fn print_history(h: &[crate::model::HistoryItem]) -> Res<()> {
    let mut last = String::new();
    let mut block = Vec::<&crate::model::HistoryItem>::new();
    for item in h {
        if item.date != last {
            if !last.is_empty() {
                print_block(&last, &block)?;
            }
            last = item.date.to_owned();
            block.clear();
        }
        block.push(item);
    }
    if !last.is_empty() {
        print_block(&last, &block)?;
    }
    Ok(())
}

fn print_block(date: &str, block: &[&crate::model::HistoryItem]) -> Res<()> {
    let d = s2t(date)?.to_rfc3339();
    println!("{d}:");
    println!(
        "{}",
        to_table(
            &block
                .iter()
                .map(|e| {
                    [
                        format!("{}", e.load),
                        format!("x"),
                        format!("{}", e.rep),
                        e.note.to_owned(),
                    ]
                })
                .collect_vec(),
        )
    );
    Ok(())
}
