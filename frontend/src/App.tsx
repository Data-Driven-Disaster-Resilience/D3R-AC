import { BrowserRouter, Routes, Route } from "react-router-dom";
import { WalletProvider } from "./context/WalletContext";
import ErrorBoundary from "./components/ErrorBoundary";
import NavBar from "./components/NavBar";
import Footer from "./components/Footer";
import Landing from "./pages/Landing";
import Dashboard from "./pages/Dashboard";
import Disburse from "./pages/Disburse";
import NotFound from "./pages/NotFound";

export default function App() {
  return (
    <ErrorBoundary>
      <WalletProvider>
        <BrowserRouter>
          <NavBar />
          <main>
            <Routes>
              <Route path="/" element={<Landing />} />
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/disburse" element={<Disburse />} />
              <Route path="*" element={<NotFound />} />
            </Routes>
          </main>
          <Footer />
        </BrowserRouter>
      </WalletProvider>
    </ErrorBoundary>
  );
}
