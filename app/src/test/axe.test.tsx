// The axe-core check of the screens is only worth something if it finds a real
// fault, so this test gives it two (a button with no name, a picture with no alt
// text) and expects both.
import { render } from "@testing-library/react";
import { expect, it } from "vitest";

import { wcagViolations } from "./backend";

it("axe finds a button with no name and a picture with no alt text", async () => {
  const { container } = render(
    <main>
      <button type="button"></button>
      <img src="a.png" />
    </main>,
  );
  const found = await wcagViolations(container);
  expect(found.map((v) => v.split(":")[0])).toEqual(expect.arrayContaining(["button-name", "image-alt"]));
});
