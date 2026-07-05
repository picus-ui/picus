use picus_widget::core::{ArcStr, NewWidget, PropertySet};
use picus_widget::parley::style::FontWeight;
use picus_widget::parley::{FontFamily, StyleProperty};
use picus_widget::peniko::Color;
use picus_widget::properties::{CaretColor, ContentColor, PlaceholderColor, SelectionColor};
use picus_widget::widgets::{self, TextAction};
use std::marker::PhantomData;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::views::Prop;
use crate::{InsertNewline, Pod, TextAlign, ViewCtx, WidgetView as _};

type Callback = Box<dyn Fn(String) + Send + Sync + 'static>;

/// A view which displays editable text.
///
/// By default, the text input is single-line - that is, pressing Enter <kbd>↵</kbd> does
/// not insert a newline.
/// This can be configured using [`insert_newline`](TextInput::insert_newline) on the
/// returned view.
/// In the default state, Enter <kbd>↵</kbd> being pressed can therefore be used as a
/// "submit" operation via [`on_enter`](TextInput::on_enter).
pub fn text_input<State, F>(contents: String, on_changed: F) -> TextInput<State>
where
    State: 'static,
    F: Fn(String) + Send + Sync + 'static,
{
    TextInput {
        contents,
        on_changed: Box::new(on_changed),
        on_enter: None,
        text_color: None,
        placeholder_color: None,
        placeholder: ArcStr::default(),
        text_alignment: TextAlign::default(),
        text_size: picus_widget::theme::TEXT_SIZE_NORMAL,
        weight: FontWeight::NORMAL,
        font: FontFamily::List(std::borrow::Cow::Borrowed(&[])),
        insert_newline: InsertNewline::default(),
        disabled: false,
        // Since we don't support setting the word wrapping, we can default to
        // not clipping
        clip: true,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`text_input`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct TextInput<State: 'static = ()> {
    contents: String,
    on_changed: Callback,
    on_enter: Option<Callback>,
    text_color: Option<Color>,
    placeholder_color: Option<Color>,
    placeholder: ArcStr,
    text_alignment: TextAlign,
    text_size: f32,
    weight: FontWeight,
    font: FontFamily<'static>,
    insert_newline: InsertNewline,
    disabled: bool,
    clip: bool,
    phantom: PhantomData<fn() -> State>,
    // TODO: add more attributes of `picus_widget::widgets::TextInput`
}

impl<State: 'static> TextInput<State> {
    /// Set the text's color.
    ///
    /// This overwrites the default `ContentColor` property for the inner `TextArea` widget.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the text color without changing the view type.
    ///
    /// `None` writes transparent text instead of falling back to the retained
    /// widget's static black.
    pub fn maybe_text_color(mut self, color: Option<Color>) -> Self {
        self.text_color = color;
        self
    }

    /// Set the insertion caret's color.
    ///
    /// This overwrites the default `CaretColor` property for the inner `TextArea` widget.
    pub fn caret_color(self, color: Color) -> Prop<CaretColor, Self, State, ()> {
        self.prop(CaretColor { color })
    }

    /// Set the selection's color.
    ///
    /// This overwrites the default `SelectionColor` property for the inner `TextArea` widget.
    pub fn selection_color(self, color: Color) -> Prop<SelectionColor, Self, State, ()> {
        self.prop(SelectionColor { color })
    }

    /// Set the string which is shown when the input is empty.
    pub fn placeholder(mut self, placeholder_text: impl Into<ArcStr>) -> Self {
        self.placeholder = placeholder_text.into();
        self
    }

    /// Set the [`PlaceholderColor`] property, which sets the color of the text shown when the input is empty.
    pub fn placeholder_color(self, color: Color) -> Prop<PlaceholderColor, Self, State, ()> {
        self.prop(PlaceholderColor::new(color))
    }

    /// Set the placeholder color without changing the view type.
    ///
    /// `None` writes transparent placeholder text instead of falling back to
    /// the retained widget's static black.
    pub fn maybe_placeholder_color(mut self, color: Option<Color>) -> Self {
        self.placeholder_color = color;
        self
    }

    /// Set the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    pub fn text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
        self
    }

    /// Sets text size.
    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Sets font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the [font family](FontFamily) this label will use.
    ///
    /// A font family allows for providing fallbacks. If there is no matching font
    /// for a character, a system font will be used (if the system fonts are enabled).
    pub fn font(mut self, font: impl Into<FontFamily<'static>>) -> Self {
        self.font = font.into();
        self
    }

    /// Configures how this text area handles the user pressing Enter <kbd>↵</kbd>.
    ///
    /// See also [`on_enter`](Self::on_enter), which provides a callback for enter
    /// being used for submitting.
    pub fn insert_newline(mut self, insert_newline: InsertNewline) -> Self {
        self.insert_newline = insert_newline;
        self
    }

    /// Set a callback that will be run when the user presses Enter <kbd>↵</kbd> to submit their input.
    ///
    /// Note that if [`insert_newline`](Self::insert_newline) is `InsertNewline::OnEnter`, this
    /// will never be called.
    pub fn on_enter<F>(mut self, on_enter: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_enter = Some(Box::new(on_enter));
        self
    }

    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the contained text will be clipped to the box if it overflows.
    ///
    /// Please note:
    /// 1) We don't currently support scrolling within a text area, so this can make some content
    ///    unviewable (without the user adding spaces and/or copy/pasting to extract content).
    ///    You should probably set this to false for small text inputs (and probably also lower
    ///    the default padding).
    /// 2) This view currently always uses word wrapping, so if there are any linebreaking
    ///    opportunities in the text, they will be taken.
    ///
    /// The default value is true (i.e. clipping is enabled).
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }
}

impl<State: 'static> ViewMarker for TextInput<State> {}
impl<State: 'static> View<State, (), ViewCtx> for TextInput<State> {
    type Element = Pod<widgets::TextInput>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        // TODO: Maybe we want a shared TextArea View?
        let text_area = widgets::TextArea::new_editable(&self.contents)
            .with_text_alignment(self.text_alignment)
            .with_insert_newline(self.insert_newline)
            .with_style(StyleProperty::FontSize(self.text_size))
            .with_style(StyleProperty::FontWeight(self.weight))
            .with_style(StyleProperty::FontFamily(self.font.clone()));

        // TODO - Replace this with properties on the TextInput view
        // once we implement property inheritance or something like it.
        let mut props = PropertySet::new();
        props.insert(ContentColor {
            color: self.text_color.unwrap_or(Color::TRANSPARENT),
        });

        let text_input =
            widgets::TextInput::from_text_area(NewWidget::new(text_area).with_props(props))
                .with_text_alignment(self.text_alignment)
                .with_clip(self.clip)
                .with_placeholder(self.placeholder.clone());

        // Ensure that the actions from the *inner* TextArea get routed correctly.
        let id = text_input.area_pod().id();
        ctx.record_action_source(id);

        let mut pod = ctx.create_pod(text_input);
        pod.new_widget.options.disabled = self.disabled;
        pod.new_widget.properties.insert(PlaceholderColor::new(
            self.placeholder_color.unwrap_or(Color::TRANSPARENT),
        ));
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        // TODO - Replace this with properties on the TextInput view
        if self.text_color != prev.text_color {
            element.insert_prop(ContentColor {
                color: self.text_color.unwrap_or(Color::TRANSPARENT),
            });
        }
        if self.placeholder != prev.placeholder {
            widgets::TextInput::set_placeholder(&mut element, self.placeholder.clone());
        }
        if self.placeholder_color != prev.placeholder_color {
            element.insert_prop(PlaceholderColor::new(
                self.placeholder_color.unwrap_or(Color::TRANSPARENT),
            ));
        }

        if self.disabled != prev.disabled {
            element.ctx.set_disabled(self.disabled);
        }

        if self.clip != prev.clip {
            widgets::TextInput::set_clip(&mut element, self.clip);
        }

        if self.text_alignment != prev.text_alignment {
            widgets::TextInput::set_text_alignment(&mut element, self.text_alignment);
        }

        let mut text_area = widgets::TextInput::text_mut(&mut element);

        // Preserve in-flight edits until the ECS-bound value actually changes.
        if self.contents != prev.contents && text_area.widget.text() != &self.contents {
            widgets::TextArea::reset_text(&mut text_area, &self.contents);
        }

        if prev.text_size != self.text_size {
            widgets::TextArea::insert_style(
                &mut text_area,
                StyleProperty::FontSize(self.text_size),
            );
        }
        if prev.weight != self.weight {
            widgets::TextArea::insert_style(&mut text_area, StyleProperty::FontWeight(self.weight));
        }
        if prev.font != self.font {
            widgets::TextArea::insert_style(
                &mut text_area,
                StyleProperty::FontFamily(self.font.clone()),
            );
        }
        if prev.insert_newline != self.insert_newline {
            widgets::TextArea::set_insert_newline(&mut text_area, self.insert_newline);
        }
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<()> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in TextInput::message"
        );
        match message.take_message::<TextAction>() {
            Some(action) => match *action {
                TextAction::Changed(text) => {
                    (self.on_changed)(text);
                    MessageResult::Action(())
                }
                TextAction::Entered(text) if self.on_enter.is_some() => {
                    (self.on_enter.as_ref().unwrap())(text);
                    MessageResult::Action(())
                }

                TextAction::Entered(_) => MessageResult::Stale,
            },
            None => {
                tracing::error!(?message, "Wrong message type in TextInput::message");
                MessageResult::Stale
            }
        }
    }
}
