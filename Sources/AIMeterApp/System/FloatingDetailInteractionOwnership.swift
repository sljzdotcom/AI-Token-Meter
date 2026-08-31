import AIMeterCore

enum FloatingDetailInteractionOwnership {
    static func accepts(
        renderedSelectionID: FloatingDetailSelectionID?,
        currentSelectionID: FloatingDetailSelectionID?
    ) -> Bool {
        guard let renderedSelectionID else { return false }
        return renderedSelectionID == currentSelectionID
    }
}
