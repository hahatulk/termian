use gpui::{
    point, prelude::*, px, rgb, App, BoxShadow, Div, FocusHandle, KeyDownEvent, Stateful, Window,
};

pub fn target(element: Stateful<Div>) -> Stateful<Div> {
    element.tab_index(0).focus(|style| {
        style.visible().shadow(vec![BoxShadow {
            color: rgb(0xa8c7ff).into(),
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(2.),
        }])
    })
}

pub fn tab(
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
    scope: Option<&FocusHandle>,
) -> bool {
    let stroke = &event.keystroke;
    let modifiers = stroke.modifiers;
    if stroke.key != "tab" || modifiers.alt || modifiers.control || modifiers.platform {
        return false;
    }
    step(window, cx, scope, modifiers.shift);
    true
}

pub fn step(window: &mut Window, cx: &mut App, scope: Option<&FocusHandle>, backwards: bool) {
    let start = window.focused(cx);
    let advance = |window: &mut Window| {
        if backwards {
            window.focus_prev()
        } else {
            window.focus_next()
        }
    };
    advance(window);
    if let Some(scope) = scope {
        let first = window.focused(cx);
        while !scope.contains_focused(window, cx) && window.focused(cx) != start {
            advance(window);
            if window.focused(cx) == first {
                break;
            }
        }
        if !scope.contains_focused(window, cx) {
            window.focus(scope);
        }
    }
    window.prevent_default();
    cx.stop_propagation();
}
