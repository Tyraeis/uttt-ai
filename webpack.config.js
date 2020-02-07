const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

const browserConfig = {
  entry: "./www/bootstrap.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin(['www/index.html'])
  ]
};

const workerConfig = {
  entry: "./www/bootstrap-worker.js",
  target: "webworker",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap-worker.js"
  },
  mode: "development"
}

module.exports = [browserConfig, workerConfig]