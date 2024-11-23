use bevy::{
    color::palettes::tailwind::*,
    ecs::world::Command,
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, TextureDimension, TextureFormat,
        },
        settings::{
            RenderCreation, WgpuFeatures, WgpuSettings,
        },
        RenderPlugin,
    },
    utils::HashMap,
};
use bevy_panorbit_camera::{
    PanOrbitCamera, PanOrbitCameraPlugin,
};
use noise::{BasicMulti, NoiseFn, Perlin, Seedable};
use std::f32::consts::PI;

fn main() {
    App::new()
        .insert_resource(TerrainStore(HashMap::default()))
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(
                    WgpuSettings {
                        // // WARN this is a native only feature. It will not work with webgl or webgpu
                        // features:
                        //     WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    },
                ),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
            PanOrbitCameraPlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (
                toggle_wireframe,
                sync_camera_to_ship,
                manage_chunks,
                control_ship,
            ),
        )
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(
            images.add(uv_debug_texture()),
        ),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 20., 75.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
        ShipCam,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: debug_material.clone(),
            transform: Transform::from_xyz(0., 10., 0.),
            ..default()
        },
        Ship,
    ));
    commands.add(SpawnTerrain(IVec2::new(-1, -1)));
    commands.add(SpawnTerrain(IVec2::new(-1, 0)));
    commands.add(SpawnTerrain(IVec2::new(-1, 1)));
    commands.add(SpawnTerrain(IVec2::new(0, -1)));
    commands.add(SpawnTerrain(IVec2::new(0, 0)));
    commands.add(SpawnTerrain(IVec2::new(0, 1)));
    commands.add(SpawnTerrain(IVec2::new(1, -1)));
    commands.add(SpawnTerrain(IVec2::new(1, 0)));
    commands.add(SpawnTerrain(IVec2::new(1, 1)));

    // directional 'sun' light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });
}

#[derive(Resource)]
struct TerrainStore(HashMap<IVec2, Handle<Mesh>>);

struct SpawnTerrain(IVec2);

impl Command for SpawnTerrain {
    fn apply(self, world: &mut World) {
        if world
            .get_resource_mut::<TerrainStore>()
            .expect("TerrainStore to be available")
            .0
            .get(&self.0)
            .is_some()
        {
            // mesh already exists
            // do nothing for now
            warn!("mesh {} already exists", self.0);
            return;
        };

        let terrain_height = 70.;
        let noise = BasicMulti::<Perlin>::new(900);
        let mesh_size = 1000.;

        let mut terrain = Mesh::from(
            Plane3d::default()
                .mesh()
                .size(mesh_size, mesh_size)
                .subdivisions(200),
        );

        if let Some(VertexAttributeValues::Float32x3(
            positions,
        )) =
            terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for pos in positions.iter_mut() {
                let val = noise.get([
                    (pos[0] as f64
                        + (mesh_size as f64
                            * self.0.x as f64))
                        / 300.,
                    (pos[2] as f64
                        + (mesh_size as f64
                            * self.0.y as f64))
                        / 300.,
                ]);

                pos[1] = val as f32 * terrain_height;
            }

            let colors: Vec<[f32; 4]> = positions
                .iter()
                .map(|[_, g, _]| {
                    let g = *g / terrain_height * 2.;

                    if g > 0.8 {
                        (Color::LinearRgba(LinearRgba {
                            red: 20.,
                            green: 20.,
                            blue: 20.,
                            alpha: 1.,
                        }))
                        .to_linear()
                        .to_f32_array()
                    } else if g > 0.3 {
                        Color::from(AMBER_800)
                            .to_linear()
                            .to_f32_array()
                    } else if g < -0.8 {
                        Color::BLACK
                            .to_linear()
                            .to_f32_array()
                    } else {
                        (Color::from(GREEN_400).to_linear())
                            .to_f32_array()
                    }
                })
                .collect();
            terrain.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                colors,
            );
        }
        terrain.compute_normals();

        let mesh = world
            .get_resource_mut::<Assets<Mesh>>()
            .expect("meshes db to be available")
            .add(terrain);
        let material = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .expect("StandardMaterial db to be available")
            .add(Color::WHITE);

        world
            .get_resource_mut::<TerrainStore>()
            .expect("TerrainStore to be available")
            .0
            .insert(self.0, mesh.clone());

        world.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_xyz(
                    self.0.x as f32 * mesh_size,
                    0.,
                    self.0.y as f32 * mesh_size,
                ),
                ..default()
            },
            Terrain,
        ));
    }
}

fn manage_chunks(
    mut commands: Commands,
    mut current_chunk: Local<IVec2>,
    ship: Query<&Transform, With<Ship>>,
    mut terrain_store: ResMut<TerrainStore>,
    terrain_entities: Query<
        (Entity, &Handle<Mesh>),
        With<Terrain>,
    >,
) {
    // same as mesh_size for us
    let chunk_size = 1000.;

    let Ok(transform) = ship.get_single() else {
        warn!("no ship!");
        return;
    };

    let xz = (transform.translation.xz() / chunk_size)
        .trunc()
        .as_ivec2();

    if *current_chunk != xz {
        *current_chunk = xz;
        let chunks_to_render = [
            *current_chunk + IVec2::new(-1, -1),
            *current_chunk + IVec2::new(-1, 0),
            *current_chunk + IVec2::new(-1, 1),
            *current_chunk + IVec2::new(0, -1),
            *current_chunk + IVec2::new(0, 0),
            *current_chunk + IVec2::new(0, 1),
            *current_chunk + IVec2::new(1, -1),
            *current_chunk + IVec2::new(1, 0),
            *current_chunk + IVec2::new(1, 1),
        ];
        // extract_if is perfect here, but its nightly
        let chunks_to_despawn: Vec<(IVec2, Handle<Mesh>)> =
            terrain_store
                .0
                .clone()
                .into_iter()
                .filter(|(key, _)| {
                    !chunks_to_render.contains(&key)
                })
                .collect();

        for (chunk, mesh) in chunks_to_despawn {
            let Some((entity, _)) = terrain_entities
                .iter()
                .find(|(_, handle)| handle == &&mesh)
            else {
                continue;
            };
            commands.entity(entity).despawn_recursive();
            terrain_store.0.remove(&chunk);
        }

        for chunk in chunks_to_render {
            commands.add(SpawnTerrain(chunk));
        }
    }
}

#[derive(Component)]
struct Terrain;

fn toggle_wireframe(
    mut commands: Commands,
    landscapes_wireframes: Query<
        Entity,
        (With<Terrain>, With<Wireframe>),
    >,
    landscapes: Query<
        Entity,
        (With<Terrain>, Without<Wireframe>),
    >,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for terrain in &landscapes {
            commands.entity(terrain).insert(Wireframe);
        }
        for terrain in &landscapes_wireframes {
            commands.entity(terrain).remove::<Wireframe>();
        }
    }
}

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct ShipCam;

fn control_ship(
    input: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mut ships: Query<&mut Transform, With<Ship>>,
) {
    let mut direction = Vec2::ZERO;

    // Keyboard support
    if input.pressed(KeyCode::KeyW) {
        direction.y += 1.;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= 1.;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x += 1.;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x -= 1.;
    }

    // Touch support
    let mut touch_data = Vec::new();
    for touch in touches.iter() {
        touch_data.push((touch.id(), touch.position(), touch.previous_position()));
    }

    if touch_data.len() == 2 {
        let (_, first_pos, first_prev_pos) = touch_data[0];
        let (_, second_pos, second_prev_pos) = touch_data[1];

        let current_midpoint = (first_pos + second_pos) / 2.0;
        let prev_midpoint = (first_prev_pos + second_prev_pos) / 2.0;
        let drag_vector = current_midpoint - prev_midpoint;

        // Use drag vector to determine ship direction
        direction.x -= drag_vector.x * 0.1; // Adjust sensitivity as needed
        direction.y += drag_vector.y * 0.1;
    }

    // Update ship position
    for mut ship in ships.iter_mut() {
        ship.translation.x += direction.x * 1.;
        ship.translation.z += direction.y * 5.;
    }
}

fn sync_camera_to_ship(
    ships: Query<
        &Transform,
        (With<Ship>, Without<ShipCam>),
    >,
    mut camera: Query<&mut PanOrbitCamera, With<ShipCam>>,
) {
    let Ok(ship) = ships.get_single() else {
        return;
    };
    let mut orbit = camera.single_mut();

    orbit.target_focus = Vec3::new(
        ship.translation.x,
        ship.translation.y,
        ship.translation.z,
    );
}

fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255,
        102, 255, 121, 255, 102, 255, 102, 255, 198, 255,
        102, 198, 255, 255, 121, 102, 255, 255, 236, 102,
        255, 255,
    ];

    let mut texture_data =
        [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)]
            .copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
