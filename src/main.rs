use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::*,
    dev_tools::fps_overlay::FpsOverlayPlugin,
    ecs::world::WorldId,
    mesh::VertexAttributeValues,
    pbr::wireframe::{Wireframe, WireframePlugin},
    platform::collections::HashMap,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{
            Extent3d, TextureDimension, TextureFormat,
        },
        settings::{
            RenderCreation, WgpuFeatures, WgpuSettings,
        },
    },
};
use bevy_malek_async::{
    AsyncPlugin, WorldIdRes, async_access,
};
use noiz::prelude::*;
use std::f32::consts::PI;

#[derive(Resource)]
pub struct TokioRuntime(tokio::runtime::Runtime);

fn main() {
    App::new()
        .insert_resource(TerrainStore(HashMap::default()))
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(
                    WgpuSettings {
                        // WARN this is a native only feature. It will not work with webgl or webgpu
                        features:
                            WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    },
                ),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin::default(),
            // PanOrbitCameraPlugin,
            AsyncPlugin,
            // FpsOverlayPlugin::default(),
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
    world_id: Res<WorldIdRes>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(
            images.add(uv_debug_texture()),
        ),
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20., 75.0)
            .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        // PanOrbitCamera::default(),
        ShipCam,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(debug_material.clone()),
        Transform::from_xyz(0., 10., 0.),
        Ship,
    ));

    let world_id = world_id.0;

    let rt = tokio::runtime::Runtime::new().unwrap();
    for chunk_position in [
        IVec2::new(-1, -1),
        IVec2::new(-1, 0),
        IVec2::new(-1, 1),
        IVec2::new(0, -1),
        IVec2::new(0, 0),
        IVec2::new(0, 1),
        IVec2::new(1, -1),
        IVec2::new(1, 0),
        IVec2::new(1, 1),
    ] {
        rt.spawn(spawn_terrain(world_id, chunk_position));
    }

    commands.insert_resource(TokioRuntime(rt));

    // directional 'sun' light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
    ));
}

#[derive(Resource)]
struct TerrainStore(HashMap<IVec2, Handle<Mesh>>);

fn manage_chunks(
    mut commands: Commands,
    mut current_chunk: Local<IVec2>,
    ship: Query<&Transform, With<Ship>>,
    mut terrain_store: ResMut<TerrainStore>,
    terrain_entities: Query<
        (Entity, &Mesh3d),
        With<Terrain>,
    >,
    world_id: WorldId,
    rt: ResMut<TokioRuntime>,
) {
    // same as mesh_size for us
    let chunk_size = 1000.;

    let Ok(transform) = ship.single() else {
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
                    !chunks_to_render.contains(key)
                })
                .collect();

        for (chunk, mesh) in chunks_to_despawn {
            let Some((entity, _)) = terrain_entities
                .iter()
                .find(|(_, handle)| ***handle == mesh)
            else {
                continue;
            };
            commands.entity(entity).despawn();
            terrain_store.0.remove(&chunk);
        }

        for chunk_position in chunks_to_render {
            rt.0.spawn(spawn_terrain(
                world_id,
                chunk_position,
            ));
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
    mut ships: Query<&mut Transform, With<Ship>>,
) {
    let mut direction = Vec2::new(0., 0.);
    if input.pressed(KeyCode::KeyW) {
        direction.y += 1.;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= 1.;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    }
    for mut ship in &mut ships {
        ship.translation.x += direction.x * 1.;
        ship.translation.z -= direction.y * 5.;
    }
}

fn sync_camera_to_ship(
    ships: Query<
        &Transform,
        (With<Ship>, Without<ShipCam>),
    >,
    mut camera: Query<&mut Transform, With<ShipCam>>,
) {
    let Ok(ship) = ships.single() else {
        return;
    };
    let mut cam = camera.single_mut().unwrap();

    cam.translation = Vec3::new(
        ship.translation.x,
        ship.translation.y + 20.,
        ship.translation.z + 75.,
    );
    cam.look_at(ship.translation, Vec3::Y);
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

async fn spawn_terrain(
    world_id: WorldId,
    chunk_position: IVec2,
) {
    let chunk_exists = async_access::<
        (Res<TerrainStore>,),
        _,
        _,
    >(
        world_id,
        |(terrain_store,)| -> bool {
            terrain_store.0.get(&chunk_position).is_some()
        },
    )
    .await;

    if chunk_exists {
        // mesh already exists
        // do nothing for now
        warn!(?chunk_position, "mesh already exists");
        return;
    }
    info!(?chunk_position, "starting generation");
    let terrain_height = 70.;
    let mut noise =
        Noise::<common_noise::Perlin>::default();
    noise.set_period(100.0);
    // let noise = BasicMulti::<Perlin>::new(900);
    let mesh_size = 1000.;

    let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(mesh_size, mesh_size)
            .subdivisions(200),
    );

    if let Some(VertexAttributeValues::Float32x3(
        positions,
    )) = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for pos in positions.iter_mut() {
            let val = noise.sample_for::<f32>(Vec2::new(
                pos[0]
                    + (mesh_size * chunk_position.x as f32),
                pos[2]
                    + (mesh_size * chunk_position.y as f32),
            ));

            pos[1] = val * terrain_height;
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
                    Color::BLACK.to_linear().to_f32_array()
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

    async_access::<
        (
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
            ResMut<TerrainStore>,
            Commands,
        ),
        _,
        _,
    >(
        world_id,
        |(
            mut meshes,
            mut materials,
            mut terrain_store,
            mut commands,
        )| {
            let mesh = meshes.add(terrain);
            let material = materials.add(Color::WHITE);

            terrain_store
                .0
                .insert(chunk_position, mesh.clone());

            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    chunk_position.x as f32 * mesh_size,
                    0.,
                    chunk_position.y as f32 * mesh_size,
                ),
                Terrain,
            ));
        },
    )
    .await;
}
