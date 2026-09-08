import AppKit
import SwiftUI

/// Keeps an editing draft until Apply or Return explicitly commits it.
struct RefreshIntervalEditor: NSViewRepresentable {
    let seconds: Int
    let save: (String) -> Bool

    func makeNSView(context: Context) -> RefreshIntervalEditorView {
        RefreshIntervalEditorView(seconds: seconds, save: save)
    }

    func updateNSView(_ view: RefreshIntervalEditorView, context: Context) {
        view.save = save
        if view.savedSeconds != seconds {
            view.savedSeconds = seconds
            view.input.stringValue = String(seconds)
            view.error.isHidden = true
        }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: RefreshIntervalEditorView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 300, height: nsView.fittingSize.height)
    }
}

final class RefreshIntervalEditorView: NSStackView {
    let input = NSTextField(string: "")
    let error = NSTextField(wrappingLabelWithString: "Enter a whole number from 30 to 86400 seconds.")
    var savedSeconds: Int
    var save: (String) -> Bool

    init(seconds: Int, save: @escaping (String) -> Bool) {
        self.savedSeconds = seconds
        self.save = save
        super.init(frame: .zero)
        orientation = .vertical
        alignment = .leading
        spacing = 6
        input.stringValue = String(seconds)
        input.font = .systemFont(ofSize: NSFont.systemFontSize)
        input.setAccessibilityLabel("Refresh interval in seconds")
        input.setAccessibilityHelp("30 to 86400 seconds. Press Return or Apply to save.")
        input.target = self
        input.action = #selector(submit)
        input.widthAnchor.constraint(equalToConstant: 90).isActive = true
        let button = NSButton(title: "Apply", target: self, action: #selector(submit))
        button.bezelStyle = .rounded
        let row = NSStackView(views: [input, NSTextField(labelWithString: "seconds"), button])
        row.spacing = 8
        addArrangedSubview(row)
        let help = NSTextField(wrappingLabelWithString: "30–86400 seconds. Default: 300 seconds (5 minutes).")
        help.font = .preferredFont(forTextStyle: .caption1)
        help.textColor = .secondaryLabelColor
        addArrangedSubview(help)
        error.font = .preferredFont(forTextStyle: .caption1)
        error.textColor = .systemRed
        error.isHidden = true
        addArrangedSubview(error)
    }

    required init?(coder: NSCoder) { nil }

    @objc private func submit() {
        // An active field editor can contain text not yet copied to its control.
        let draft = (input.currentEditor() as? NSTextView)?.string ?? input.stringValue
        if save(draft) {
            input.stringValue = String(Int(draft.trimmingCharacters(in: .whitespacesAndNewlines))!)
            error.isHidden = true
        } else {
            error.isHidden = false
        }
    }
}
