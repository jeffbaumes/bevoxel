use crate::voxel::MaterialRegistry;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySlot {
    pub material_name: String,
    pub quantity: u32,
}

impl InventorySlot {
    pub fn new(material_name: impl Into<String>, quantity: u32) -> Self {
        Self {
            material_name: material_name.into(),
            quantity,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.quantity == 0 || self.material_name == "air"
    }

    pub fn add(&mut self, amount: u32) {
        self.quantity = self.quantity.saturating_add(amount);
    }

    pub fn remove(&mut self, amount: u32) -> u32 {
        let removed = amount.min(self.quantity);
        self.quantity -= removed;
        if self.quantity == 0 {
            self.material_name = "air".to_string();
        }
        removed
    }

    pub fn can_remove(&self, amount: u32) -> bool {
        self.quantity >= amount && !self.is_empty()
    }
}

impl Default for InventorySlot {
    fn default() -> Self {
        Self::new("air", 0)
    }
}

#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<InventorySlot>,
    pub rows: usize,
    pub columns: usize,
    pub selected_slot: usize,
}

impl Inventory {
    pub fn new(rows: usize, columns: usize) -> Self {
        let total_slots = rows * columns;
        let mut slots = Vec::with_capacity(total_slots);
        for _ in 0..total_slots {
            slots.push(InventorySlot::default());
        }

        Self {
            slots,
            rows,
            columns,
            selected_slot: 0,
        }
    }

    pub fn get_slot(&self, index: usize) -> Option<&InventorySlot> {
        self.slots.get(index)
    }

    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut InventorySlot> {
        self.slots.get_mut(index)
    }

    pub fn get_selected_slot(&self) -> &InventorySlot {
        &self.slots[self.selected_slot]
    }

    pub fn get_selected_slot_mut(&mut self) -> &mut InventorySlot {
        &mut self.slots[self.selected_slot]
    }

    pub fn move_selection(&mut self, row_delta: i32, col_delta: i32) {
        let current_row = self.selected_slot / self.columns;
        let current_col = self.selected_slot % self.columns;

        let new_row = ((current_row as i32 + row_delta).rem_euclid(self.rows as i32)) as usize;
        let new_col = ((current_col as i32 + col_delta).rem_euclid(self.columns as i32)) as usize;

        self.selected_slot = new_row * self.columns + new_col;
    }

    pub fn add_material(&mut self, material_name: &str, quantity: u32) -> u32 {
        if material_name == "air" || quantity == 0 {
            return 0;
        }

        let mut remaining = quantity;

        // First try to add to existing stacks of the same material
        for slot in &mut self.slots {
            if slot.material_name == material_name && !slot.is_empty() {
                let can_add = u32::MAX - slot.quantity;
                let to_add = remaining.min(can_add);
                slot.add(to_add);
                remaining -= to_add;
                if remaining == 0 {
                    return quantity;
                }
            }
        }

        // Then try to add to empty slots
        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.material_name = material_name.to_string();
                let to_add = remaining.min(u32::MAX);
                slot.quantity = to_add;
                remaining -= to_add;
                if remaining == 0 {
                    return quantity;
                }
            }
        }

        quantity - remaining
    }

    pub fn remove_material(&mut self, material_name: &str, quantity: u32) -> u32 {
        if material_name == "air" || quantity == 0 {
            return 0;
        }

        let mut remaining = quantity;

        for slot in &mut self.slots {
            if slot.material_name == material_name && !slot.is_empty() {
                let removed = slot.remove(remaining);
                remaining -= removed;
                if remaining == 0 {
                    return quantity;
                }
            }
        }

        quantity - remaining
    }

    pub fn has_material(&self, material_name: &str, quantity: u32) -> bool {
        if material_name == "air" || quantity == 0 {
            return true;
        }

        let mut total = 0;
        for slot in &self.slots {
            if slot.material_name == material_name && !slot.is_empty() {
                total += slot.quantity;
                if total >= quantity {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_material_count(&self, material_name: &str) -> u32 {
        if material_name == "air" {
            return u32::MAX;
        }

        let mut total = 0;
        for slot in &self.slots {
            if slot.material_name == material_name && !slot.is_empty() {
                total += slot.quantity;
            }
        }

        total
    }

    pub fn initialize_with_test_content(&mut self) {
        // Add some test materials to different slots
        if let Some(slot) = self.get_slot_mut(0) {
            *slot = InventorySlot::new("stone", 64);
        }
        if let Some(slot) = self.get_slot_mut(1) {
            *slot = InventorySlot::new("dirt", 32);
        }
        if let Some(slot) = self.get_slot_mut(2) {
            *slot = InventorySlot::new("grass", 16);
        }
        if let Some(slot) = self.get_slot_mut(3) {
            *slot = InventorySlot::new("wood", 8);
        }
        if let Some(slot) = self.get_slot_mut(5) {
            *slot = InventorySlot::new("glass", 12);
        }
        if let Some(slot) = self.get_slot_mut(7) {
            *slot = InventorySlot::new("sand", 24);
        }
    }
}

#[derive(Component)]
pub struct InventoryUI;

#[derive(Component)]
pub struct InventorySlotUI {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct InventorySlotBackground;

#[derive(Component)]
pub struct InventorySlotQuantity;

#[derive(Component)]
pub struct InventorySlotSelection;

pub fn setup_inventory_ui(commands: &mut Commands, inventory: &Inventory) {
    let slot_size = 60.0;
    let slot_spacing = 5.0;
    let total_width = inventory.columns as f32 * slot_size + (inventory.columns - 1) as f32 * slot_spacing;
    let total_height = inventory.rows as f32 * slot_size + (inventory.rows - 1) as f32 * slot_spacing;

    // Main inventory container
    commands
        .spawn((
            Node {
                width: Val::Px(total_width),
                height: Val::Px(total_height),
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(inventory.columns as u16, 1.0),
                grid_template_rows: RepeatedGridTrack::flex(inventory.rows as u16, 1.0),
                column_gap: Val::Px(slot_spacing),
                row_gap: Val::Px(slot_spacing),
                ..default()
            },
            GlobalZIndex(2000),
            InventoryUI,
        ))
        .with_children(|parent| {
            // Create inventory slots
            for row in 0..inventory.rows {
                for col in 0..inventory.columns {
                    let slot_index = row * inventory.columns + col;
                    let is_selected = slot_index == inventory.selected_slot;

                    parent
                        .spawn((
                            Node {
                                width: Val::Px(slot_size),
                                height: Val::Px(slot_size),
                                border: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::End,
                                align_items: AlignItems::End,
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            BorderColor(if is_selected { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) }),
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                            InventorySlotUI { slot_index },
                            InventorySlotBackground,
                        ))
                        .with_children(|slot_parent| {
                            // Quantity text
                            slot_parent.spawn((
                                Text::new(""),
                                TextColor(Color::WHITE),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                InventorySlotQuantity,
                            ));
                        });
                }
            }
        });
}

pub fn update_inventory_ui(
    inventory: Res<Inventory>,
    material_registry: Res<MaterialRegistry>,
    mut slot_query: Query<(Entity, &InventorySlotUI, &mut BackgroundColor, &mut BorderColor), With<InventorySlotBackground>>,
    mut quantity_query: Query<&mut Text, With<InventorySlotQuantity>>,
    children_query: Query<&Children>,
) {
    if !inventory.is_changed() {
        return;
    }

    // Update slot backgrounds and borders
    for (entity, slot_ui, mut bg_color, mut border_color) in slot_query.iter_mut() {
        let is_selected = slot_ui.slot_index == inventory.selected_slot;
        border_color.0 = if is_selected { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) };

        if let Some(slot) = inventory.get_slot(slot_ui.slot_index) {
            if !slot.is_empty() {
                let material = material_registry.get(&slot.material_name);
                let material_color = material.get_color();
                bg_color.0 = Color::srgba(
                    material_color.to_srgba().red * 0.7,
                    material_color.to_srgba().green * 0.7,
                    material_color.to_srgba().blue * 0.7,
                    0.8,
                );
            } else {
                bg_color.0 = Color::srgba(0.1, 0.1, 0.1, 0.8);
            }
        }

        // Update quantity text for this slot
        if let Ok(children) = children_query.get(entity) {
            for &child in children.iter() {
                if let Ok(mut text) = quantity_query.get_mut(child) {
                    if let Some(slot) = inventory.get_slot(slot_ui.slot_index) {
                        if !slot.is_empty() {
                            text.0 = slot.quantity.to_string();
                        } else {
                            text.0 = String::new();
                        }
                    }
                }
            }
        }
    }
}

pub fn handle_inventory_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
) {
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        inventory.move_selection(-1, 0);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        inventory.move_selection(1, 0);
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        inventory.move_selection(0, -1);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        inventory.move_selection(0, 1);
    }
}