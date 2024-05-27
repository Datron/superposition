use crate::components::pagination::pagination::Pagination;

use super::types::{Column, TablePaginationProps};
use leptos::*;
use serde_json::{json, Map, Value};

fn generate_table_row_str(row: &Value) -> String {
    match row {
        Value::Null => "null".to_string(),
        Value::String(rstr) => rstr.to_string(),
        Value::Number(rnum) => rnum.to_string(),
        Value::Bool(rbool) => rbool.to_string(),
        Value::Array(rarr) => rarr
            .iter()
            .map(|ele| generate_table_row_str(ele))
            .collect::<Vec<String>>()
            .join(","),
        Value::Object(robj) => json!(robj).to_string(),
    }
}

#[component]
pub fn table(
    key_column: String,
    cell_style: String,
    columns: Vec<Column>,
    rows: Vec<Map<String, Value>>,
    #[prop(default = TablePaginationProps::default())] pagination: TablePaginationProps,
) -> impl IntoView {
    let pagination_props = StoredValue::new(pagination);
    view! {
        <div class="overflow-x-auto">
            <table class="table table-zebra">
                <thead>
                    <tr>
                        <th></th>

                        {columns
                            .iter()
                            .filter(|column| !column.hidden)
                            .map(|column| {
                                view! {
                                    <th class="uppercase">{&column.name.replace("_", " ")}</th>
                                }
                            })
                            .collect_view()}

                    </tr>
                </thead>
                <tbody>

                    {rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| {
                            let row_id = row
                                .get(&key_column)
                                .unwrap_or(&json!(""))
                                .as_str()
                                .unwrap()
                                .to_string();
                            let TablePaginationProps { enabled, current_page, count, .. } = pagination_props
                                .get_value();
                            let row_num = if enabled {
                                index as i64 + 1 + ((current_page - 1) * count)
                            } else {
                                index as i64 + 1
                            };
                            view! {
                                <tr id=row_id>
                                    <th>{row_num}</th>

                                    {columns
                                        .iter()
                                        .filter(|column| !column.hidden)
                                        .map(|column| {
                                            let cname = &column.name;
                                            let value: String = generate_table_row_str(
                                                row.get(cname).unwrap_or(&Value::String("".to_string())),
                                            );
                                            view! {
                                                <td class=cell_style
                                                    .to_string()>{(column.formatter)(&value, &row)}</td>
                                            }
                                        })
                                        .collect_view()}

                                </tr>
                            }
                        })
                        .collect_view()}

                </tbody>
            </table>
            <Show when=move || {
                pagination_props.get_value().enabled
            }>

                {move || {
                    let TablePaginationProps { current_page, total_pages, on_prev, on_next, .. } = pagination_props
                        .get_value();
                    view! {
                        <div class="mt-2 flex justify-end">
                            <Pagination
                                current_page=current_page
                                total_pages=total_pages
                                next=on_next
                                previous=on_prev
                            />
                        </div>
                    }
                }}

            </Show>
        </div>
    }
}
