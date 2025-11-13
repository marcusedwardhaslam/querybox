const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const mainConfig = {
  mode: 'development',
  entry: './src/main/main.ts',
  target: 'electron-main',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'main.js'
  },
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/
      }
    ]
  },
  node: {
    __dirname: false,
    __filename: false
  }
};

const rendererConfig = {
  mode: 'development',
  entry: './src/renderer/renderer.ts',
  target: 'electron-renderer',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'renderer.js'
  },
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/
      },
      {
        test: /\.scss$/,
        use: ['style-loader', 'css-loader', 'sass-loader']
      },
      {
        test: /\.css$/,
        use: ['style-loader', 'css-loader']
      }
    ]
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: './src/renderer/index.html',
      filename: 'index.html'
    })
  ]
};

module.exports = [mainConfig, rendererConfig];
