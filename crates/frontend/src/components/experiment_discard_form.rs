pub mod utils;

use std::rc::Rc;

use leptos::*;
use utils::discard_experiment;
use web_sys::MouseEvent;

use crate::{
    components::button::Button,
    types::{Experiment, OrganisationId, Tenant},
};

#[component]
pub fn experiment_discard_form<NF>(
    experiment: Experiment,
    handle_submit: NF,
) -> impl IntoView
where
    NF: Fn() + 'static + Clone,
{
    let (change_reason, set_change_reason) = create_signal(String::new());
    let tenant_rws = use_context::<RwSignal<Tenant>>().unwrap();
    let org_rws = use_context::<RwSignal<OrganisationId>>().unwrap();
    let (req_inprogess_rs, req_inprogress_ws) = create_signal(false);
    let experiment_rc = Rc::new(experiment);

    let handle_discard_experiment = move |event: MouseEvent| {
        req_inprogress_ws.set(true);
        event.prevent_default();
        let experiment_clone = experiment_rc.clone();
        let handle_submit_clone = handle_submit.clone();
        spawn_local(async move {
            let tenant = tenant_rws.get().0;
            let org = org_rws.get().0;
            let change_reason_value = change_reason.get();
            let _ = discard_experiment(
                &experiment_clone.id,
                &tenant,
                &org,
                &change_reason_value,
            )
            .await;
            req_inprogress_ws.set(false);
            handle_submit_clone()
        });
    };

    view! {
        <h3 class="font-bold text-lg">Discard Experiment</h3>
        <p class="py-4">Safely discard the experiment without affecting any pre-existing overrides</p>
        <form>
            <div class="form-control pb-4">
                <label class="label">
                    <span class="label-text">Reason for Change</span>
                </label>
                <textarea
                    placeholder="Enter a reason for this change"
                    class="textarea textarea-bordered w-full max-w-md"
                    value=change_reason.get_untracked()
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        set_change_reason.set(value);
                    }
                />
            </div>

            {move || {
                let loading = req_inprogess_rs.get();
                view! {
                    <Button text="Discard".to_string() on_click=handle_discard_experiment.clone() loading/>
                }
            }}

        </form>
    }
}
