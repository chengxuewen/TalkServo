import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, theme } from "antd";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { DispatcherPage } from "./pages/dispatcher.js";
import { FieldPage } from "./pages/field.js";
import "./main.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ConfigProvider
      theme={{
        algorithm: theme.darkAlgorithm,
        token: { colorPrimary: "#3b82f6" },
      }}
    >
      <BrowserRouter>
        <Routes>
          <Route path="/d/:room" element={<DispatcherPage />} />
          <Route path="/f/:room" element={<FieldPage />} />
          <Route path="*" element={<Navigate to="/f/demo" replace />} />
        </Routes>
      </BrowserRouter>
    </ConfigProvider>
  </React.StrictMode>,
);
