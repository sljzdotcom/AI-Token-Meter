import "@testing-library/jest-dom/vitest"
import { cleanup } from "@testing-library/react"
import { afterEach } from "vitest"
import { setLocale } from "../localization"

afterEach(() => { cleanup(); setLocale("en") })
// jsdom has no rasterizer. Font measurement itself is covered with an injected measurer
// and the production browser typography gate exercises real canvas/font rendering.
Object.defineProperty(HTMLCanvasElement.prototype, "getContext", { value: () => null })
