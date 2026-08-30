import { loadConfig } from "./config.js";
import { initLogging, logger } from "./logging.js";
import { createState } from "./app_state.js";
import { createServer } from "./api/http.js";

const config = loadConfig();
initLogging(config.logDir);
const state = createState(config);
const server = createServer(state);

server.listen(config.port, config.host, () => {
  logger.info("api.listen", { host: config.host, port: config.port });
  console.log(`AISetu UI  http://127.0.0.1:${config.port}/`);
  console.log(`OpenAI     http://127.0.0.1:${config.port}/v1`);
});

function shutdown() {
  logger.info("shutdown", {});
  server.close(() => process.exit(0));
  setTimeout(() => process.exit(0), 1500);
}
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
