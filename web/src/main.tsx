import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, theme } from "antd";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { DispatcherPage } from "./pages/dispatcher.js";
import { FieldPage } from "./pages/field.js";
import { ParticipantPage } from "./pages/participant.js";
import { AdminPage } from "./pages/admin.js";
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
          <Route path="/dispatch/:room" element={<DispatcherPage />} />
          <Route path="/field/:room" element={<FieldPage />} />
          <Route path="/room/:room" element={<ParticipantPage />} />
          <Route path="/admin" element={<AdminPage />} />
          <Route path="*" element={<Navigate to="/field/demo" replace />} />
        </Routes>
      </BrowserRouter>
    </ConfigProvider>
  </React.StrictMode>,
);
