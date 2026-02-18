mod colloscope_sheet;
mod formats;
mod groups_sheet;

use std::path::Path;

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::group_lists::GroupListParameters;
use rust_xlsxwriter::{Workbook, XlsxError};

pub fn write_xlsx(inner_data: &InnerData, path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();

    let colloscope_ws = workbook.add_worksheet();
    colloscope_ws.set_name("Colloscope")?;
    colloscope_sheet::build(colloscope_ws, &inner_data.params, &inner_data.colloscope)?;

    let groups_ws = workbook.add_worksheet();
    groups_ws.set_name("Groupes")?;
    groups_sheet::build(groups_ws, &inner_data.params, &inner_data.colloscope)?;

    workbook.save(path)?;
    Ok(())
}

pub(crate) fn get_group_name(params: &GroupListParameters, group_num: u32) -> String {
    let idx = group_num as usize;
    if let Some(Some(name)) = params.group_names.get(idx) {
        return name.to_string();
    }
    (group_num + 1).to_string()
}
