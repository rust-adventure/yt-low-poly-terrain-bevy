use std::f32::consts::PI;

use bevy::{
    color::palettes::tailwind::*,
    core_pipeline::{
        bloom::BloomSettings,
        dof::{DepthOfFieldMode, DepthOfFieldSettings},
        prepass::{DepthPrepass, NormalPrepass},
        tonemapping::Tonemapping,
        Skybox,
    },
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
    render::mesh::VertexAttributeValues,
};
use bevy_panorbit_camera::{
    PanOrbitCamera, PanOrbitCameraPlugin,
};
use noise::{BasicMulti, NoiseFn, Perlin, Seedable};
mod water_material;
use water_material::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            MaterialPlugin::<
                ExtendedMaterial<
                    StandardMaterial,
                    WaterExtension,
                >,
            >::default(),
        ))
        .add_systems(Startup, startup)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<
        Assets<
            ExtendedMaterial<
                StandardMaterial,
                WaterExtension,
            >,
        >,
    >,
    asset_server: ResMut<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 20., 75.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                tonemapping: Tonemapping::TonyMcMapface,
            ..default()
        },
        // Skybox {
        //     brightness: 5000.0,
        //     image: asset_server.load(
        //         "skybox_cubemap/skybox_specular.ktx2",
        //     ),
        // },
        // EnvironmentMapLight {
        //     diffuse_map: asset_server
        //         .load("skybox_cubemap/skybox_diffuse.ktx2"),
        //     specular_map: asset_server.load(
        //         "skybox_cubemap/skybox_specular.ktx2",
        //     ),
        //     intensity: 2000.0,
        // },
        Skybox {
            brightness: 1000.0,
            image: asset_server.load(
                "kloppenheim_06_puresky_4k_diffuse/kloppenheim_06_puresky_4k_specular.ktx2",
            ),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("kloppenheim_06_puresky_4k_diffuse/kloppenheim_06_puresky_4k_diffuse.ktx2"),
            specular_map: asset_server.load(
                "kloppenheim_06_puresky_4k_diffuse/kloppenheim_06_puresky_4k_specular.ktx2",
            ),
            intensity: 1000.0,
        },
        BloomSettings::NATURAL,
        PanOrbitCamera::default(),
        DepthOfFieldSettings{
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: 40.,
            aperture_f_stops: 1.0 / 8.0,
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
    ));

    let terrain_height = 70.;

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
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        // cascade_shadow_config: CascadeShadowConfigBuilder {
        //     first_cascade_far_bound: 4.0,
        //     maximum_distance: 10.0,
        //     ..default()
        // }
        // .into(),
        ..default()
    });

    let noise = BasicMulti::<Perlin>::default();

    let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(1000.0, 1000.0)
            .subdivisions(200),
    );

    if let Some(VertexAttributeValues::Float32x3(
        positions,
    )) = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for pos in positions.iter_mut() {
            pos[1] = noise.get([
                pos[0] as f64 / 300.,
                pos[2] as f64 / 300.,
                0. as f64,
            ]) as f32
                * terrain_height;
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
                    Color::from(GREEN_400)
                        .to_linear()
                        .to_f32_array()
                }
            })
            .collect();
        terrain.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            colors,
        );

        terrain.compute_normals();
    }

    commands.spawn(PbrBundle {
        mesh: meshes.add(terrain),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        }),
        ..default()
    });

    // water
    let water = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(1000.0, 1000.0)
            .subdivisions(200),
    );

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(water),
        transform: Transform::from_xyz(
            0.,
            -(terrain_height / 2.)
                + terrain_height * 6. / 16.,
            0.,
        ),
        material: water_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                clearcoat: 1.0,
                clearcoat_perceptual_roughness: 0.3,
                // clearcoat_normal_texture: Some(asset_server.load_with_settings(
                //     "textures/ScratchedGold-Normal.png",
                //     |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
                // )),
                metallic: 0.9,
                // mine
                base_color: BLUE_400.into(),
                perceptual_roughness: 0.2,
                // can be used in forward or deferred mode.
                opaque_render_method:
                    OpaqueRendererMethod::Auto,
                // in deferred mode, only the PbrInput can be modified (uvs, color and other material properties),
                // in forward mode, the output can also be modified after lighting is applied.
                // see the fragment shader `extended_material.wgsl` for more info.
                // Note: to run in deferred mode, you must also add a `DeferredPrepass` component to the camera and either
                // change the above to `OpaqueRendererMethod::Deferred` or add the `DefaultOpaqueRendererMethod` resource.
                alpha_mode: AlphaMode::Blend,
                ..default()
            },
            extension: WaterExtension {
                quantize_steps: 30,
            },
        }),
        ..default()
    });
}
