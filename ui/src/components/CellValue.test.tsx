import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CellValue } from "./CellValue";

describe("CellValue", () => {
  it.each([
    "https://shelf.example/books/2",
    "http://127.0.0.1:8080/books?id=2#top",
    "HTTPS://SHELF.EXAMPLE/",
  ])(
    "renders %s as a link that opens in a new tab without telling that tab where it came from",
    (url) => {
      render(<CellValue value={url} />);

      const link = screen.getByRole("link", { name: url });
      expect(link).toHaveAttribute("href", url);
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).toHaveAttribute("rel", "noopener noreferrer");
    },
  );

  it.each([
    "javascript:alert(1)",
    "data:text/html,<p>hello</p>",
    "file:///etc/passwd",
    "ftp://shelf.example/books",
    "mailto:reader@shelf.example",
    "https:shelf.example",
    "https:/shelf.example",
    "//shelf.example/books",
    "shelf.example/books",
    "https://",
    "see https://shelf.example/books",
    "https://shelf.example/books and more",
    " https://shelf.example/books",
    "https://shelf.example/books\n",
    "https://%",
    "https:///shelf.example",
    "https:////x",
    "https://\\shelf.example",
    "https://shelf.example/\u202Ebooks",
    "https://shelf.example/books\u0001",
  ])("shows %j as text because it is not wholly an absolute http or https URL", (text) => {
    const { container } = render(<CellValue value={text} />);

    expect(screen.queryByRole("link")).toBeNull();
    expect(container.textContent).toBe(text);
  });

  it("still shows a duration in words beside the value the tool printed", () => {
    render(<CellValue value="PT1H40M" />);

    expect(screen.getByText("PT1H40M")).toBeVisible();
    expect(screen.getByText(/1 h 40 min/)).toBeVisible();
  });
});
