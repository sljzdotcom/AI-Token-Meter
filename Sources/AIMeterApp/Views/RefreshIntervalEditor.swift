import AppKit
import SwiftUI

/// Keeps an editing draft until Apply or Return explicitly commits it.
struct RefreshIntervalEditor: NSViewRepresentable {
    let seconds: Int
    let save: (String) -> Bool
    @Environment(\.locale) private var locale

    private var localizer: AppLocalizer { AppLocalizer(language: AppLanguage(rawValue: locale.identifier) ?? .english) }

    func makeNSView(context: Context) -> RefreshIntervalEditorView {
        RefreshIntervalEditorView(seconds: seconds, save: save, localizer: localizer)
    }

    func updateNSView(_ view: RefreshIntervalEditorView, context: Context) {
        view.updateLocalization(localizer)
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
    let error = NSTextField(wrappingLabelWithString: "")
    private let button = NSButton()
    private let unit = NSTextField(labelWithString: "")
    private let help = NSTextField(wrappingLabelWithString: "")
    var savedSeconds: Int
    var save: (String) -> Bool

    init(seconds: Int, save: @escaping (String) -> Bool, localizer: AppLocalizer = AppLocalizer(language: .english)) {
        self.savedSeconds = seconds
        self.save = save
        super.init(frame: .zero)
        orientation = .vertical
        alignment = .leading
        spacing = 6
        input.stringValue = String(seconds)
        input.font = .systemFont(ofSize: NSFont.systemFontSize)
        input.target = self
        input.action = #selector(submit)
        // Losing focus keeps the draft; only Return or Apply commits it.
        (input.cell as? NSTextFieldCell)?.sendsActionOnEndEditing = false
        input.widthAnchor.constraint(equalToConstant: 90).isActive = true
        button.target = self
        button.action = #selector(submit)
        button.bezelStyle = .rounded
        let row = NSStackView(views: [input, unit, button])
        row.spacing = 8
        addArrangedSubview(row)
        help.font = .preferredFont(forTextStyle: .caption1)
        help.textColor = .secondaryLabelColor
        addArrangedSubview(help)
        error.font = .preferredFont(forTextStyle: .caption1)
        error.textColor = .systemRed
        error.isHidden = true
        addArrangedSubview(error)
        updateLocalization(localizer)
    }

    func updateLocalization(_ localizer: AppLocalizer) {
        input.setAccessibilityLabel(localizer.text("Refresh interval in seconds"))
        input.setAccessibilityHelp(localizer.text("30 to 86400 seconds. Press Return or Apply to save."))
        button.title = localizer.text("Apply")
        unit.stringValue = localizer.text("seconds")
        help.stringValue = localizer.text("30–86400 seconds. Default: 300 seconds (5 minutes).")
        error.stringValue = localizer.text("Enter a whole number from 30 to 86400 seconds.")
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
