import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { applyDemoParams } from "./lib/demo";
import { app as state } from "./lib/state.svelte";

applyDemoParams();
void state.boot();

const app = mount(App, { target: document.getElementById("app")! });

export default app;
