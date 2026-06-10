import React from "react";
import { 
  Network
} from "lucide-react";
import VirtualLANEngine from "./components/VirtualLANEngine";

export default function App() {
  return (
    <div className="min-h-screen bg-[#F8FAFC] text-[#1E293B] flex flex-col font-sans select-none antialiased">
      
      {/* Top Header Bar */}
      <header className="border-b border-slate-200 bg-white sticky top-0 z-50 px-4 sm:px-6 py-4">
        <div className="max-w-7xl mx-auto flex justify-between items-center">
          
          {/* Logo and Server status indicator */}
          <div className="flex items-center gap-3 text-left flex-row">
            <div className="w-10 h-10 rounded-xl bg-[#EA580C] flex items-center justify-center shadow-sm shrink-0">
              <Network className="text-white" size={20} />
            </div>
            <div>
              <h1 className="font-extrabold text-[#1E293B] tracking-tight text-base sm:text-lg font-sans">
                RustNet • Virtual LAN Adapter Simulator
              </h1>
              <p className="text-[10px] text-slate-400 font-mono mt-0.5">
                P2P Virtual LAN Tunnel Client & Coordinator Hub
              </p>
            </div>
          </div>

        </div>
      </header>

      {/* Main Container Area */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-4 sm:p-6 space-y-6">
        
        {/* Active Client Engine Dashboard */}
        <VirtualLANEngine />

      </main>

      {/* Footer omitting all system internal logs to prevent slop */}
      <footer className="border-t border-slate-200 bg-white py-6 mt-auto text-center text-xs text-slate-400 font-sans">
        <div className="max-w-xl mx-auto px-4 space-y-2">
          <p className="leading-relaxed">
            RustNet L2 Virtual Tunnel System • Secure P2P Bridge Simulation without physical port forwarding.
          </p>
          <p className="font-semibold text-slate-500 text-sm">
            Developed by Mobin
          </p>
        </div>
      </footer>

    </div>
  );
}
