// Render tests for the shared badge/progress vocabulary.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Badge } from "./Badge";
import { ProgressBar } from "./ProgressBar";

describe("Badge", () => {
  it("renders the spill badge with a count", () => {
    render(<Badge kind="spill" count={3} />);
    expect(screen.getByText(/spilled ×3/)).toBeTruthy();
  });

  it("omits the count when it is one", () => {
    render(<Badge kind="spill" count={1} />);
    expect(screen.getByText(/^⮔spilled$/)).toBeTruthy();
  });
});

describe("ProgressBar", () => {
  it("shows the percentage paired with the SP fraction", () => {
    render(<ProgressBar done={5} inProgress={3} total={10} />);
    expect(screen.getByText("50% — 5/10 SP")).toBeTruthy();
  });

  it("handles a zero denominator without NaN", () => {
    render(<ProgressBar done={0} total={0} />);
    expect(screen.getByText("0% — 0/0 SP")).toBeTruthy();
  });
});
