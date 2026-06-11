import { NavLink } from "react-router-dom";

const ITEMS = [
  { to: "/", glyph: "◧", label: "Home", end: true },
  { to: "/epics", glyph: "▤", label: "Epics" },
  { to: "/sprints", glyph: "▥", label: "Sprints" },
  { to: "/spillover", glyph: "⮔", label: "Spillover" },
  { to: "/settings", glyph: "⚙", label: "Settings" },
];

export function Sidebar() {
  return (
    <nav className="sidebar">
      <div className="brand">
        Increment <span>Tracker</span>
      </div>
      {ITEMS.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.end}
          className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}
        >
          <span className="glyph">{item.glyph}</span>
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}
