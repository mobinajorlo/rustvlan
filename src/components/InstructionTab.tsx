import React, { useState } from "react";
import { Terminal, Copy, Check, Info, Settings, ShieldAlert, Cpu } from "lucide-react";
import { rustAgentExplanation, cargoTomlCode, mainRsCode } from "../rustAgentCode";

export default function InstructionTab() {
  const [lang, setLang] = useState<"fa" | "en">("en");
  const [copiedFile, setCopiedFile] = useState<"cargo" | "main" | null>(null);

  const copyText = (text: string, file: "cargo" | "main") => {
    navigator.clipboard.writeText(text);
    setCopiedFile(file);
    setTimeout(() => setCopiedFile(null), 2000);
  };

  const exp = rustAgentExplanation[lang];

  return (
    <div className="space-y-6">
      {/* Header and Language Selector */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 bg-white p-4 rounded-xl border border-slate-200 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-orange-50 text-[#EA580C] rounded-lg border border-orange-100">
            <Cpu size={20} />
          </div>
          <div>
            <h3 className="font-semibold text-[#1E293B] font-sans text-sm sm:text-base">
              {lang === "fa" ? "مستندات بازنویسی دیمون کلاینت به Rust" : "Rust Daemon Client Documentation"}
            </h3>
            <p className="text-xs text-slate-500 mt-0.5">
              {lang === "fa" ? "رفع باگ عدم شناسایی در بازی‌ها و بازدهی حداکثر" : "Fixing in-game detection bugs and achieving maximum throughput"}
            </p>
          </div>
        </div>
        <div className="flex bg-slate-50 p-1 rounded-lg border border-slate-200 self-end sm:self-auto shadow-inner">
          <button
            onClick={() => setLang("fa")}
            className={`px-3 py-1 text-xs rounded-md transition-all font-sans cursor-pointer ${
              lang === "fa"
                ? "bg-white text-[#EA580C] font-semibold shadow-sm"
                : "text-slate-500 hover:text-slate-800"
            }`}
          >
            Farsi (فارسی)
          </button>
          <button
            onClick={() => setLang("en")}
            className={`px-3 py-1 text-xs rounded-md transition-all font-sans cursor-pointer ${
              lang === "en"
                ? "bg-white text-[#EA580C] font-semibold shadow-sm"
                : "text-slate-500 hover:text-slate-800"
            }`}
          >
            English
          </button>
        </div>
      </div>

      {/* Persian/English Bug Explanations */}
      <div className={`${lang === "fa" ? "text-right" : "text-left"} bg-white p-6 rounded-xl border border-slate-200 shadow-sm space-y-4`}>
        <h4 className="text-lg font-bold text-[#1E293B] flex items-center gap-2 font-sans justify-start">
          <ShieldAlert className="text-[#EA580C] shrink-0" size={20} />
          <span>{exp.title}</span>
        </h4>
        <p className="text-sm text-slate-600 leading-relaxed font-sans">{exp.intro}</p>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2">
          {exp.reasons.map((r, index) => (
            <div key={index} className="bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-2">
              <h5 className="text-sm font-semibold text-[#EA580C] flex items-center gap-1.5 font-sans justify-start">
                <span className="w-2 h-2 rounded-full bg-[#EA580C]"></span>
                {r.title}
              </h5>
              <p className="text-xs text-slate-500 leading-relaxed font-sans">{r.desc}</p>
            </div>
          ))}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
          <div className="bg-orange-50/50 p-4 rounded-xl border border-orange-100 space-y-2 text-right">
            <h4 className={`text-sm font-bold text-[#EA580C] font-sans flex items-center gap-1.5 ${lang === "fa" ? "justify-end" : "justify-start"}`}>{exp.solveTitle}</h4>
            <p className="text-xs text-slate-600 leading-relaxed font-sans">{exp.solveDesc}</p>
          </div>
          <div className="bg-emerald-50/70 p-4 rounded-xl border border-emerald-100 space-y-2 text-right">
            <h4 className={`text-sm font-bold text-[#059669] font-sans flex items-center gap-1.5 ${lang === "fa" ? "justify-end" : "justify-start"}`}>
              <span>{exp.cleanupTitle}</span>
            </h4>
            <p className="text-xs text-slate-600 leading-relaxed font-sans">{exp.cleanupDesc}</p>
          </div>
        </div>
      </div>

      {/* Code viewer tabs - styled with premium dark code terminals inside clean layout */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Cargo.toml Section */}
        <div className="flex flex-col bg-[#1E293B] rounded-xl border border-slate-800 overflow-hidden shadow-sm">
          <div className="flex justify-between items-center p-3 bg-slate-800/80 border-b border-slate-800">
            <span className="text-xs font-mono text-slate-200 flex items-center gap-2">
              <Settings size={14} className="text-slate-400" /> Cargo.toml (Dependencies)
            </span>
            <button
               onClick={() => copyText(cargoTomlCode, "cargo")}
               className="text-slate-350 hover:text-[#EA580C] transition-colors p-1.5 bg-[#1E293B] rounded border border-slate-800 cursor-pointer"
               title="Copy Cargo.toml"
            >
              {copiedFile === "cargo" ? <Check size={14} className="text-emerald-400" /> : <Copy size={13} />}
            </button>
          </div>
          <pre className="p-4 overflow-x-auto text-[11px] font-mono text-slate-300 leading-normal max-h-[300px] select-all bg-[#1E293B]">
            <code>{cargoTomlCode}</code>
          </pre>
        </div>

        {/* main.rs Section */}
        <div className="flex flex-col bg-[#1E293B] rounded-xl border border-slate-800 overflow-hidden shadow-sm">
          <div className="flex justify-between items-center p-3 bg-slate-800/80 border-b border-slate-800">
            <span className="text-xs font-mono text-slate-200 flex items-center gap-2">
              <Terminal size={14} className="text-slate-400" /> src/main.rs (Full Layer-2 Adapter Client)
            </span>
            <button
              onClick={() => copyText(mainRsCode, "main")}
              className="text-slate-350 hover:text-[#EA580C] transition-colors p-1.5 bg-[#1E293B] rounded border border-slate-800 cursor-pointer"
              title="Copy main.rs"
            >
              {copiedFile === "main" ? <Check size={14} className="text-emerald-400" /> : <Copy size={13} />}
            </button>
          </div>
          <pre className="p-4 overflow-x-auto text-[11px] font-mono text-slate-300 leading-normal max-h-[300px] select-all bg-[#1E293B]">
            <code>{mainRsCode}</code>
          </pre>
        </div>
      </div>

      {/* Guide steps to run local Rust Agent */}
      <div className="bg-white p-5 rounded-xl border border-slate-200 space-y-4 shadow-sm text-right">
        <h4 className="text-sm font-semibold text-[#1E293B] flex items-center gap-2 font-sans justify-end">
          <span>{lang === "fa" ? "مراحل گام‌به‌گام راه‌اندازی مأمور Rust روی پردازنده شما" : "Step-by-step Local Compilation Setup Guide"}</span>
          <Info size={16} className="text-[#EA580C]" />
        </h4>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs font-sans text-right">
          <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-2">
            <div className="w-6 h-6 bg-orange-100 text-[#EA580C] flex items-center justify-center rounded-full font-bold ml-auto">1</div>
            <p className="font-semibold text-[#1E293B]">
              {lang === "fa" ? "ایجاد پوشه جدید" : "Create cargo repository"}
            </p>
            <p className="text-slate-500 leading-relaxed">
              {lang === "fa" 
                ? "یک دایرکتوری در ترمینال بسازید و دستور cargo init را بزنید. سپس دو فایل Cargo.toml و src/main.rs را با گزینه‌های بالا پر کنید." 
                : "Initialize a new project using cargo init, then copy-paste Cargo.toml and main.rs content above into respective workspaces."}
            </p>
          </div>

          <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-2">
            <div className="w-6 h-6 bg-orange-100 text-[#EA580C] flex items-center justify-center rounded-full font-bold ml-auto">2</div>
            <p className="font-semibold text-[#1E293B]">
              {lang === "fa" ? "نصب درایور محلی TAP" : "Install local TAP driver"}
            </p>
            <p className="text-slate-500 leading-relaxed">
              {lang === "fa" 
                ? "برای ویندوز نرم‌افزار OpenVPN Tap را نصب کنید تا کارت شبکه مجازی موسوم به 'RustNetTAP' شناخته شود. در لینوکس دسترسی sudo کافی‌ست."
                : "For Windows, install OpenVPN Desktop Tap client to seed the TAP adapter virtual interface named 'RustNetTAP'. For Linux, tap0 generates automatically via standard superuser privileges."}
            </p>
          </div>

          <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-2">
            <div className="w-6 h-6 bg-orange-100 text-[#EA580C] flex items-center justify-center rounded-full font-bold ml-auto">3</div>
            <p className="font-semibold text-[#1E293B]">
              {lang === "fa" ? "کامپایل و اتصال پرسرعت" : "Compile and Execute"}
            </p>
            <p className="text-slate-500 leading-relaxed">
              {lang === "fa"
                ? "دستور زیر را با آدرس سرور این وب‌اپلیکیشن به همراه شناسه روم سفارشی خود اجرا کنید:\n\ncargo run -- -s ws://ADDRESS -r ROOM_NAME -u USERNAME"
                : "Execute cargo compilation using the terminal targeting our central web application coordinator:\n\ncargo run -- -s ws://YOUR_APPLET_URL -r ROOM_ID"}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
