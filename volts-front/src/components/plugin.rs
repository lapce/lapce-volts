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
            class = "w-full md:w-1/2 lg:w-1/3 p-4"
        ) {
            li(
                class="flex border rounded-md py-4 w-full"
            ) {
                img(
                    class="m-4 h-auto w-16",
                    src="https://raw.githubusercontent.com/lapce/lapce/master/extra/images/logo.png",
                ) {}
                div(class="flex flex-col justify-between w-[calc(100%-6rem)] pr-4") {
                    div {
                        p(
                            class="font-bold"
                        ) {
                            (plugin.plugin.display_name)
                        }
                        p(
                            class="mt-1 text-ellipsis whitespace-nowrap overflow-hidden"
                        ) {
                            (plugin.plugin.description)
                        }
                    }
                    div(
                        class="flex justify-between text-sm text-gray-400 mt-3"
                    ) {
                        p {
                            (plugin.plugin.author)
                        }
                        p {
                            "Downloads: " (plugin.plugin.downloads)
                        }
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
        div(class="container m-auto") {
            h1(class="mt-3 mx-3 font-bold") {
                "Most Downloaded"
            }
            ul(class="flex flex-wrap") {
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
}
