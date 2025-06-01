use colorgetter::ui::{Ui, UiError};

fn main() -> Result<(), UiError> {
    let ui = Ui::try_new()?;

    let gs = ui.setup_menu_loop::<15, 15>()?;
    let solution = ui.solution_finding_loop(&gs)?;
    if let Some(solution) = solution {
        ui.solution_viewer_loop(&solution)?;
    }

    Ok(())
}
