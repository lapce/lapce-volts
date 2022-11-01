use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::{view, Keyed},
    reactive::{create_signal, Scope, Signal},
    view::View,
    web::Html,
};
use volts_core::{EncodePlugin, PluginList};

#[derive(PartialEq, Eq, Clone)]
struct IndexedPlugin {
    plugin: EncodePlugin,
    last: bool,
}

fn get_plugins<'a>(cx: Scope<'a>, q: &str, plugins: &'a Signal<Vec<IndexedPlugin>>) {
    let req = Request::get("/api/v1/plugins").send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let plugin_list: PluginList = resp.json().await.unwrap();
        let len = plugin_list.plugins.len();
        let plugin_list = plugin_list
            .plugins
            .into_iter()
            .enumerate()
            .map(|(i, plugin)| IndexedPlugin {
                plugin,
                last: i + 1 == len,
            })
            .collect();
        plugins.set(plugin_list);
    });
}

#[component(inline_props)]
fn PluginItem<'a, G: Html>(
    cx: Scope<'a>,
    plugin: IndexedPlugin,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
) -> View<G> {
    view! {cx,
        div(
            class = "w-full md:w-1/2 lg:w-1/3 px-4 border rounded-md text-center"
        ) {
            li(
                class=""
            ) {
                img(
                    class="inline-flex my-4 h-16 w-16",
                    src="https://raw.githubusercontent.com/lapce/lapce/master/extra/images/logo.png",
                ) {}
                p(
                    class="font-bold"
                ) {
                    (plugin.plugin.display_name)
                }
                p(
                    class="text-ellipsis whitespace-nowrap overflow-hidden w-full"
                ) {
                    (plugin.plugin.description)
                }
                div(
                    class="flex justify-between text-sm text-gray-200"
                ) {
                    p {
                        (plugin.plugin.downloads)
                    }
                    p {
                        "Downloads: " (plugin.plugin.downloads)
                    }
                }
            }
        }
    }
}

#[component]
pub fn PluginList<G: Html>(cx: Scope) -> View<G> {
    let plugins = create_signal(cx, Vec::new());
    get_plugins(cx, "", plugins);
    view! {cx,
        ul(class="") {
            Keyed(
                iterable=plugins,
                view=move |cx, plugin| view! {cx,
                    PluginItem(plugin=plugin, plugins=plugins)
                },
                key=|plugin| plugin.plugin.name.clone(),
            )
        }
    }
}
