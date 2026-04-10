(function() {
  const { Editor, EditorState } = Draft;

  class App extends React.Component {
    constructor(props) {
      super(props);
      this.state = { editorState: EditorState.createEmpty() };
      this.onChange = (editorState) => this.setState({ editorState });
    }
    render() {
      return React.createElement(Editor, {
        editorState: this.state.editorState,
        onChange: this.onChange,
        placeholder: 'Type or paste here...',
      });
    }
  }

  const root = ReactDOM.createRoot(document.getElementById('root'));
  root.render(React.createElement(App));
})();
