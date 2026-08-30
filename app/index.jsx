import { AppRegistry } from "react-native";
import App from "./App.jsx";

AppRegistry.registerComponent("AISetu", () => App);
AppRegistry.runApplication("AISetu", {
  rootTag: document.getElementById("root"),
});
