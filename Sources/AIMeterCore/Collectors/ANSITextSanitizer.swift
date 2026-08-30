import Foundation

public enum ANSITextSanitizer {
    public static func sanitize(_ text: String) -> String {
        let withoutOSC = text.replacingOccurrences(
            of: "\\u001B\\].*?(?:\\u0007|\\u001B\\\\)",
            with: "",
            options: .regularExpression
        )
        let withoutCSI = withoutOSC.replacingOccurrences(
            of: "\\u001B\\[[0-?]*[ -/]*[@-~]",
            with: "",
            options: .regularExpression
        )

        var scalars: [UnicodeScalar] = []
        for scalar in withoutCSI.unicodeScalars {
            if scalar.value == 0x08 {
                _ = scalars.popLast()
            } else if scalar.value == 0x0D {
                if scalars.last?.value != 0x0A {
                    scalars.append("\n")
                }
            } else if scalar.value == 0x0A || scalar.value == 0x09 || scalar.value >= 0x20 {
                scalars.append(scalar)
            }
        }

        return String(String.UnicodeScalarView(scalars))
            .replacingOccurrences(of: #"\n{2,}"#, with: "\n", options: .regularExpression)
    }
}
