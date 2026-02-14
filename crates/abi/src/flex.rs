//! # Flex Layout

use crate::{
    Axis, ChildElement, Element, ElementBuilder, LayoutPass, Length, LengthRequest, MeasureContext,
    Xy,
};



pub struct Flex {
    axis: Axis,
    main_alignment: AxisAlignment,
    cross_alignment: CrossAlignment,
    elements: Vec<FlexElement>,
}

enum FlexElement {
    Child {
        element: ChildElement,
        alignment: Option<CrossAlignment>,
        flex: f32,
        basis: Option<FlexBasis>,

        resolved_basis: f32,
    },
    Spacer {
        flex: f32,
        basis: f32,

        resolved_basis: f32,
        resolved_length: f32,
    },
}

impl Flex {
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            main_alignment: AxisAlignment::Start,
            cross_alignment: CrossAlignment::Center,
            elements: Vec::new(),
        }
    }

    #[inline]
    pub fn row() -> Self {
        Self::new(Axis::Horizontal)
    }

    #[inline]
    pub fn column() -> Self {
        Self::new(Axis::Vertical)
    }

    pub fn with(mut self, child: impl Element + 'static, params: impl Into<FlexParams>) -> Self {
        let params = params.into();
        self.elements.push(FlexElement::Child {
            element: ElementBuilder::new(child).into_child(),
            alignment: params.alignment,
            flex: params.flex,
            basis: params.basis,
            resolved_basis: 0.0,
        });
        self
    }

    pub fn with_spacer(mut self, flex: f32) -> Self {
        self.elements.push(FlexElement::Spacer {
            flex,
            basis: 0.0,
            resolved_basis: 0.0,
            resolved_length: 0.0,
        });
        self
    }

    pub fn with_main_align(mut self, alignment: AxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    pub fn with_cross_align(mut self, alignment: CrossAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }
}

impl Element for Flex {
    fn children_ids(&self) -> Vec<u64> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                FlexElement::Child { element, .. } => Some(element.id()),
                FlexElement::Spacer { .. } => None,
            })
            .collect()
    }

    fn update_children(&mut self, pass: &mut crate::UpdatePass<'_>) {
        for element in self.elements.iter_mut() {
            let FlexElement::Child { element, .. } = element else {
                continue;
            };
            pass.update_child(element);
        }
    }

    #[cfg(debug_assertions)]
    fn render(&mut self, pass: &mut crate::RenderPass<'_>) {
        use crate::{Aabb2D, Rgba};

        let bounds = pass.bounds();
        let baseline = bounds.size().y - pass.baseline_offset();

        pass.fill_quad(
            Aabb2D::new(
                bounds.min.x,
                bounds.min.y + (baseline - 1.0),
                bounds.max.x,
                bounds.min.y + (baseline + 1.0),
            ),
            Rgba::rgb(0x73, 0x73, 0x89),
            0.0,
            Rgba::NONE,
        );
    }

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        let gap_length = 3.0; // self.gap;
        let gap_count = self.elements.len().saturating_sub(1);

        let size = pass.size;
        let main_axis = self.axis;
        let cross_axis = main_axis.cross();
        let cross_space = size.value_for_axis(cross_axis);

        let mut main_space: f32 = size.value_for_axis(main_axis) - gap_count as f32 * gap_length;
        let mut max_ascent: f32 = 0.0;
        let mut flex_sum: f32 = 0.0;
        let mut lowest_baseline: f32 = f32::INFINITY;

        let resolve_child_size =
            |pass: &mut LayoutPass<'_>,
             child: &mut ChildElement,
             child_main_length: f32,
             alignment: &Option<CrossAlignment>| {
                let cross_auto = match alignment.unwrap_or(self.cross_alignment) {
                    CrossAlignment::Stretch => Length::Exact(cross_space),
                    _ => Length::FitContent(cross_space),
                };

                let child_cross_length = pass.resolve_length(
                    child.id(),
                    cross_axis,
                    cross_auto,
                    Some(child_main_length),
                );

                main_axis.pack_xy(child_main_length, child_cross_length)
            };
        let mut do_child_layout =
            |pass: &mut LayoutPass<'_>, child: &mut ChildElement, child_size: Xy<f32>| {
                pass.do_layout(child, child_size);

                let baseline = pass
                    .expect_child(child.id())
                    .element
                    .state
                    .layout_baseline_offset;
                let ascent = child_size.y - baseline;
                max_ascent = max_ascent.max(ascent);
            };
        let mut place_child =
            |pass: &mut LayoutPass<'_>, child: &mut ChildElement, child_origin: Xy<f32>| {
                pass.place_child(child, child_origin);

                let child_node = pass.expect_child(child.id());
                let child_size = child_node.element.state.layout_bounds.size();
                let child_baseline = child_node.element.state.layout_baseline_offset;
                let child_bottom = child_origin.y + child_size.y;
                let bottom_gap = size.y - child_bottom;
                let baseline = child_baseline + bottom_gap;
                lowest_baseline = lowest_baseline.min(baseline);
            };

        // Add up flex factors, resolve bases, subtract bases from main space, and lay
        // out inflexible elements.
        for child in &mut self.elements {
            match child {
                FlexElement::Child {
                    element,
                    alignment,
                    flex,
                    basis,
                    resolved_basis,
                } => {
                    match effective_basis(*basis, *flex) {
                        FlexBasis::Auto => {
                            // Basis is always resolved with a `MaxContent` fallback.
                            let main_fallback = Length::MaxContent;
                            *resolved_basis = pass.resolve_length(
                                element.id(),
                                main_axis,
                                main_fallback,
                                Some(cross_space),
                            );
                            main_space -= *resolved_basis;
                        }
                        FlexBasis::Zero => {
                            *resolved_basis = 0.0;
                        }
                    }
                    if *flex == 0.0 {
                        let child_main_length = *resolved_basis;
                        let child_size =
                            resolve_child_size(pass, element, child_main_length, alignment);

                        do_child_layout(pass, element, child_size);
                    } else {
                        flex_sum += *flex;
                    }
                }
                FlexElement::Spacer {
                    flex,
                    basis,
                    resolved_basis,
                    resolved_length,
                } => {
                    *resolved_basis = *basis; // * scale;
                    main_space -= *resolved_basis;

                    if *flex == 0.0 {
                        *resolved_length = *resolved_basis;
                    } else {
                        flex_sum += *flex;
                    }
                }
            }
        }

        // Calculate the flex fraction, i.e. the amount of space per one flex factor.
        let flex_fraction = if flex_sum > 0.0 {
            main_space.max(0.0) / flex_sum
        } else {
            0.0
        };

        // Offer the available space to flexible children.
        for child in &mut self.elements {
            match child {
                FlexElement::Child {
                    element,
                    alignment,
                    flex,
                    resolved_basis,
                    ..
                } if *flex > 0.0 => {
                    let child_main_length = *resolved_basis + *flex * flex_fraction;
                    let child_size =
                        resolve_child_size(pass, element, child_main_length, alignment);

                    do_child_layout(pass, element, child_size);

                    main_space -= child_main_length - *resolved_basis;
                }
                FlexElement::Spacer {
                    flex,
                    resolved_basis,
                    resolved_length,
                    ..
                } if *flex > 0.0 => {
                    let child_main_length = *resolved_basis + *flex * flex_fraction;
                    *resolved_length = child_main_length;
                    main_space -= *resolved_length - *resolved_basis;
                }
                _ => (),
            }
        }

        // We only distribute free space around elements, not spacers.
        let element_count = self
            .elements
            .iter()
            .filter(|element| matches!(element, FlexElement::Child { .. }))
            .count();
        let (space_before, space_between) =
            get_spacing(self.main_alignment, main_space.max(0.0), element_count);

        // Distribute free space and place children.
        let mut main_offset = space_before;
        let mut previous_was_element = false;
        for child in &mut self.elements {
            match child {
                FlexElement::Child {
                    element, alignment, ..
                } => {
                    if previous_was_element {
                        main_offset += space_between;
                    }

                    let child_node = pass.expect_child(element.id());
                    let child_size = child_node.element.state.layout_bounds.size();
                    let alignment = alignment.unwrap_or(self.cross_alignment);
                    let child_origin_cross = match alignment {
                        CrossAlignment::Baseline if main_axis == Axis::Horizontal => {
                            let baseline = child_node.element.state.layout_baseline_offset;
                            let ascent = child_size.y - baseline;
                            max_ascent - ascent
                        }
                        _ => {
                            let cross_unused = cross_space - child_size.value_for_axis(cross_axis);
                            alignment.offset(cross_unused)
                        }
                    };

                    let child_origin = main_axis.pack_xy(main_offset, child_origin_cross);
                    place_child(pass, element, child_origin);

                    main_offset += child_size.value_for_axis(main_axis);
                    main_offset += gap_length;
                    previous_was_element = true;
                }
                FlexElement::Spacer {
                    resolved_length, ..
                } => {
                    main_offset += *resolved_length;
                    main_offset += gap_length;
                    previous_was_element = false;
                }
            }
        }

        // If we have at least one child then we can use the lowest child baseline.
        let baseline = self
            .elements
            .iter()
            .any(|element| matches!(element, FlexElement::Child { .. }))
            .then_some(lowest_baseline);

        if let Some(baseline) = baseline {
            pass.set_baseline_offset(baseline);
        } else {
            pass.clear_baseline_offset();
        }
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        measure_axis: Axis,
        length_request: LengthRequest,
        perpendicular_length: Option<f32>,
    ) -> f32 {
        let perpendicular_axis = measure_axis.cross();
        let main_axis = self.axis;
        let cross_axis = main_axis.cross();
        let gap_length = 3.0; // self.gap;
        let gap_count = self.elements.len().saturating_sub(1);

        let (main_space, cross_space) = if perpendicular_axis == main_axis {
            (perpendicular_length, None)
        } else {
            (None, perpendicular_length)
        };

        let (length_request, min_result) = match length_request {
            LengthRequest::MinContent | LengthRequest::MaxContent => (length_request, 0.0),
            // We always want to use up all offered space but may need even more,
            // so we implement FitContent as space.max(MinContent).
            LengthRequest::FitContent(space) => (LengthRequest::MinContent, space),
        };

        // We can skip resolving bases if we don't know the main space when measuring
        // cross. This is because in that code path we don't ever read the
        // resolved basis.
        let skip_resolving_bases = measure_axis == cross_axis && main_space.is_none();
        if !skip_resolving_bases {
            // Basis is always resolved with a `MaxContent` fallback.
            let main_fallback = Length::MaxContent;

            for child in &mut self.elements {
                match child {
                    FlexElement::Child {
                        element,
                        flex,
                        basis,
                        resolved_basis,
                        ..
                    } => match effective_basis(*basis, *flex) {
                        FlexBasis::Auto => {
                            *resolved_basis = context.resolve_length(
                                element.id(),
                                main_axis,
                                main_fallback,
                                cross_space,
                            );
                        }
                        FlexBasis::Zero => {
                            *resolved_basis = 0.0;
                        }
                    },
                    FlexElement::Spacer {
                        basis,
                        resolved_basis,
                        ..
                    } => {
                        *resolved_basis = *basis; // * scale;
                    }
                }
            }
        }

        let mut length = 0.0;
        if measure_axis == main_axis {
            // Find the largest desired flex fraction.
            let mut flex_fraction: f32 = 0.0;
            let main_fallback = length_request.into();
            for child in &mut self.elements {
                let desired_flex_fraction = match child {
                    FlexElement::Child {
                        element,
                        flex,
                        basis,
                        ..
                    } => {
                        if *flex > 0.0 {
                            match effective_basis(*basis, *flex) {
                                FlexBasis::Auto => {
                                    // Auto basis is always MaxContent, so this child doesn't want
                                    // any extra flex space regardless of whether the request is min
                                    // or max.
                                    0.0
                                }
                                FlexBasis::Zero => {
                                    let child_length = context.resolve_length(
                                        element.id(),
                                        main_axis,
                                        main_fallback,
                                        cross_space,
                                    );
                                    // Flexible children with a zero basis want to reach their
                                    // target lengths purely with flex space.
                                    child_length / *flex
                                }
                            }
                        } else {
                            // Inflexible children remain at their basis sizes, and don't want any
                            // extra flex space.
                            0.0
                        }
                    }
                    FlexElement::Spacer { .. } => {
                        // Spacer basis fully covers its preferred size, so spacers don't want any
                        // extra flex space.
                        0.0
                    }
                };
                flex_fraction = flex_fraction.max(desired_flex_fraction);
            }

            // Calculate the total space needed for all children.
            length += self
                .elements
                .iter()
                .map(|child| match child {
                    FlexElement::Child {
                        flex,
                        resolved_basis,
                        ..
                    }
                    | FlexElement::Spacer {
                        flex,
                        resolved_basis,
                        ..
                    } => *resolved_basis + *flex * flex_fraction,
                })
                .sum::<f32>();

            // Add all the gap lengths.
            length += gap_count as f32 * gap_length;
        } else {
            // If we know the main axis space, then we can distribute it to children. This
            // is important, because some elements need it for accurate measurement.
            let flex_fraction = main_space.map(|mut main_space| {
                // Add up flex factors and subtract bases from main space.
                let mut flex_sum = 0.0;
                for child in &mut self.elements {
                    match child {
                        FlexElement::Child {
                            flex,
                            resolved_basis,
                            ..
                        }
                        | FlexElement::Spacer {
                            flex,
                            resolved_basis,
                            ..
                        } => {
                            flex_sum += *flex;
                            main_space -= *resolved_basis;
                        }
                    }
                }

                // Subtract gap lengths.
                main_space -= gap_count as f32 * gap_length;

                // Calculate the flex fraction, i.e. the amount of space per one flex factor
                if flex_sum > 0.0 {
                    main_space.max(0.0) / flex_sum
                } else {
                    0.0
                }
            });

            // Calculate the total space needed for all children
            for child in &mut self.elements {
                match child {
                    FlexElement::Child {
                        element,
                        flex,
                        resolved_basis,
                        ..
                    } => {
                        let child_main_length = flex_fraction
                            .map(|flex_fraction| *resolved_basis + *flex * flex_fraction);
                        let cross_auto = length_request.into();

                        let child_cross_length = context.resolve_length(
                            element.id(),
                            cross_axis,
                            cross_auto,
                            child_main_length,
                        );

                        length = length.max(child_cross_length);
                    }
                    // Spacers don't contribute to cross length
                    FlexElement::Spacer { .. } => (),
                }
            }

            // Gaps don't contribute to the cross axis
        }

        min_result.max(length)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlexParams {
    flex: f32,
    basis: Option<FlexBasis>,
    alignment: Option<CrossAlignment>,
}

impl From<f32> for FlexParams {
    fn from(value: f32) -> Self {
        Self {
            flex: value,
            basis: None,
            alignment: None,
        }
    }
}

fn effective_basis(basis: Option<FlexBasis>, flex: f32) -> FlexBasis {
    basis.unwrap_or(if flex == 0.0 {
        FlexBasis::Auto
    } else {
        FlexBasis::Zero
    })
}

/// Calculates `(space_before, space_between)` from the `extra` space given the
/// `child_count`.
fn get_spacing(alignment: AxisAlignment, extra: f32, child_count: usize) -> (f32, f32) {
    let space_before;
    let space_between;
    match alignment {
        _ if child_count == 0 => {
            space_before = 0.0;
            space_between = 0.0;
        }
        AxisAlignment::Start => {
            space_before = 0.0;
            space_between = 0.0;
        }
        AxisAlignment::End => {
            space_before = extra;
            space_between = 0.0;
        }
        AxisAlignment::Center => {
            space_before = extra / 2.0;
            space_between = 0.0;
        }
        AxisAlignment::SpaceBetween => {
            let equal_space = extra / child_count.saturating_sub(1) as f32;
            space_before = 0.0;
            space_between = equal_space;
        }
        AxisAlignment::SpaceEvenly => {
            let equal_space = extra / (child_count + 1) as f32;
            space_before = equal_space;
            space_between = equal_space;
        }
        AxisAlignment::SpaceAround => {
            let equal_space = extra / (2 * child_count) as f32;
            space_before = equal_space;
            space_between = equal_space * 2.0;
        }
    }

    (space_before, space_between)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    #[default]
    Auto,
    Zero,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CrossAlignment {
    Start,
    Center,
    End,
    Baseline,
    Stretch,
}

impl CrossAlignment {
    pub fn offset(self, space: f32) -> f32 {
        match self {
            Self::Start => 0.0,
            Self::Center | Self::Baseline => space / 2.0,
            Self::End => space,
            Self::Stretch => 0.0,
        }
    }
}
